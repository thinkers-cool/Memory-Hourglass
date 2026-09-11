use crate::catalog::repo::{AssetMetaRepo, AssetRepo, RawTagRepo};
use crate::error::{AppError, Result};
use crate::metadata::MetadataContext;
use crate::scan::index_asset::{index_asset_on_disk, resolved_thumb_key, IndexOutput};
use crate::scan::index_integrity::is_index_complete;
use sqlx::SqlitePool;
use std::path::Path;

pub struct IndexApplyInput {
    pub asset_id: i64,
    pub mtime_ns: i64,
    pub kind: String,
    pub prior_thumb_key: Option<String>,
    pub indexed: IndexOutput,
}

pub async fn apply_index_output(pool: &SqlitePool, input: &IndexApplyInput) -> Result<()> {
    let raw_tag_repo = RawTagRepo::new(pool.clone());
    let meta_repo = AssetMetaRepo::new(pool.clone());
    let assets = AssetRepo::new(pool.clone());

    raw_tag_repo
        .replace_for_asset(input.asset_id, &input.indexed.raw_tags)
        .await?;
    meta_repo.upsert(&input.indexed.meta).await?;
    if let Some(key) = &input.indexed.thumb_key {
        assets.set_thumb_key(input.asset_id, key).await?;
    }
    let thumb_key = resolved_thumb_key(
        &input.kind,
        input.indexed.thumb_key.as_deref(),
        input.prior_thumb_key.as_deref(),
    );
    if is_index_complete(input.mtime_ns, Some(input.mtime_ns), thumb_key, &input.kind) {
        assets.mark_indexed(input.asset_id, input.mtime_ns).await?;
    }
    Ok(())
}

pub async fn index_asset_and_apply(
    pool: &SqlitePool,
    asset_id: i64,
    path: &Path,
    thumb_dir: &Path,
    mtime_ns: i64,
    kind: &str,
    prior_thumb_key: Option<String>,
) -> Result<IndexOutput> {
    let path_buf = path.to_path_buf();
    let thumb_dir_buf = thumb_dir.to_path_buf();
    let kind_str = kind.to_string();
    let indexed = tokio::task::spawn_blocking(move || {
        if std::env::var_os("MEMHG_TEST_INDEX_BATCH_PANIC").is_some() {
            std::env::remove_var("MEMHG_TEST_INDEX_BATCH_PANIC");
            panic!("index batch panic");
        }
        let metadata_ctx = MetadataContext::in_place(path_buf.clone());
        index_asset_on_disk(asset_id, &path_buf, &thumb_dir_buf, &metadata_ctx)
    })
    .await
    .map_err(|e| AppError::Scan(e.to_string()))?;

    apply_index_output(
        pool,
        &IndexApplyInput {
            asset_id,
            mtime_ns,
            kind: kind_str,
            prior_thumb_key,
            indexed: IndexOutput {
                meta: indexed.meta.clone(),
                raw_tags: indexed.raw_tags.clone(),
                thumb_key: indexed.thumb_key.clone(),
            },
        },
    )
    .await?;

    Ok(indexed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::repo::{AssetRepo, SourceRootRepo};
    use crate::catalog::Catalog;
    use crate::scan::index_asset::IndexOutput;
    use crate::scan::index_integrity::is_index_complete;
    use tempfile::tempdir;

    #[tokio::test]
    async fn apply_index_output_marks_asset_indexed() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let pool = catalog.pool().clone();
        let roots = SourceRootRepo::new(pool.clone());
        let root = roots
            .insert_root(dir.path().to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        let assets = AssetRepo::new(pool.clone());
        let asset = assets
            .upsert_asset(crate::catalog::repo::UpsertAssetInput {
                root_id: root.id,
                rel_path: "photo.jpg",
                file_name: "photo.jpg",
                ext: "jpg",
                kind: "image",
                size: 1,
                mtime_ns: 42,
                sync_state: "ok",
            })
            .await
            .unwrap();

        let thumb_dir = dir.path().join("thumbs");
        std::fs::create_dir_all(&photos_dir(&dir)).unwrap();
        std::fs::write(
            photos_dir(&dir).join("photo.jpg"),
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();

        let indexed = index_asset_and_apply(
            &pool,
            asset.id,
            &photos_dir(&dir).join("photo.jpg"),
            &thumb_dir,
            42,
            "image",
            None,
        )
        .await
        .unwrap();
        assert_eq!(indexed.meta.asset_id, asset.id);

        let updated = assets.get_asset(asset.id).await.unwrap();
        assert_eq!(updated.indexed_mtime_ns, Some(42));
    }

    #[tokio::test]
    async fn apply_index_output_without_thumb_skips_mark_indexed() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let pool = catalog.pool().clone();
        let roots = SourceRootRepo::new(pool.clone());
        let root = roots
            .insert_root(dir.path().to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        let assets = AssetRepo::new(pool.clone());
        let asset = assets
            .upsert_asset(crate::catalog::repo::UpsertAssetInput {
                root_id: root.id,
                rel_path: "video.mp4",
                file_name: "video.mp4",
                ext: "mp4",
                kind: "video",
                size: 1,
                mtime_ns: 99,
                sync_state: "ok",
            })
            .await
            .unwrap();

        let output = IndexOutput {
            meta: crate::catalog::models::AssetMeta {
                asset_id: asset.id,
                capture_at: None,
                camera: None,
                lens: None,
                rating: None,
                latitude: None,
                longitude: None,
                keywords_json: None,
            },
            raw_tags: vec![],
            thumb_key: None,
        };
        apply_index_output(
            &pool,
            &IndexApplyInput {
                asset_id: asset.id,
                mtime_ns: 99,
                kind: "video".into(),
                prior_thumb_key: None,
                indexed: output,
            },
        )
        .await
        .unwrap();

        let updated = assets.get_asset(asset.id).await.unwrap();
        assert!(updated.indexed_mtime_ns.is_none());
        assert!(!is_index_complete(99, Some(99), None, "video"));
    }

    fn photos_dir(dir: &tempfile::TempDir) -> std::path::PathBuf {
        dir.path().join("photos")
    }

    #[tokio::test]
    async fn index_asset_and_apply_tolerates_missing_file() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let pool = catalog.pool().clone();
        let roots = SourceRootRepo::new(pool.clone());
        let root = roots
            .insert_root(dir.path().to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        let assets = AssetRepo::new(pool.clone());
        let asset = assets
            .upsert_asset(crate::catalog::repo::UpsertAssetInput {
                root_id: root.id,
                rel_path: "missing.jpg",
                file_name: "missing.jpg",
                ext: "jpg",
                kind: "image",
                size: 1,
                mtime_ns: 1,
                sync_state: "ok",
            })
            .await
            .unwrap();
        let indexed = index_asset_and_apply(
            &pool,
            asset.id,
            &dir.path().join("missing.jpg"),
            &dir.path().join("thumbs"),
            1,
            "image",
            None,
        )
        .await
        .unwrap();
        assert_eq!(indexed.meta.asset_id, asset.id);
        assert!(indexed.meta.capture_at.is_none());
        assert!(indexed.thumb_key.is_none());
    }

    #[tokio::test]
    async fn apply_index_output_uses_prior_thumb_for_completeness() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let pool = catalog.pool().clone();
        let roots = SourceRootRepo::new(pool.clone());
        let root = roots
            .insert_root(dir.path().to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        let assets = AssetRepo::new(pool.clone());
        let asset = assets
            .upsert_asset(crate::catalog::repo::UpsertAssetInput {
                root_id: root.id,
                rel_path: "cached.jpg",
                file_name: "cached.jpg",
                ext: "jpg",
                kind: "image",
                size: 1,
                mtime_ns: 55,
                sync_state: "ok",
            })
            .await
            .unwrap();
        let output = IndexOutput {
            meta: crate::catalog::models::AssetMeta {
                asset_id: asset.id,
                capture_at: None,
                camera: None,
                lens: None,
                rating: None,
                latitude: None,
                longitude: None,
                keywords_json: None,
            },
            raw_tags: vec![],
            thumb_key: None,
        };
        apply_index_output(
            &pool,
            &IndexApplyInput {
                asset_id: asset.id,
                mtime_ns: 55,
                kind: "image".into(),
                prior_thumb_key: Some("cached.webp".into()),
                indexed: output,
            },
        )
        .await
        .unwrap();
        let updated = assets.get_asset(asset.id).await.unwrap();
        assert_eq!(updated.indexed_mtime_ns, Some(55));
    }

    #[tokio::test]
    async fn apply_index_output_sets_thumb_key_when_present() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let pool = catalog.pool().clone();
        let roots = SourceRootRepo::new(pool.clone());
        let root = roots
            .insert_root(dir.path().to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        let assets = AssetRepo::new(pool.clone());
        let asset = assets
            .upsert_asset(crate::catalog::repo::UpsertAssetInput {
                root_id: root.id,
                rel_path: "thumb.jpg",
                file_name: "thumb.jpg",
                ext: "jpg",
                kind: "image",
                size: 1,
                mtime_ns: 77,
                sync_state: "ok",
            })
            .await
            .unwrap();
        apply_index_output(
            &pool,
            &IndexApplyInput {
                asset_id: asset.id,
                mtime_ns: 77,
                kind: "image".into(),
                prior_thumb_key: None,
                indexed: IndexOutput {
                    meta: crate::catalog::models::AssetMeta {
                        asset_id: asset.id,
                        capture_at: None,
                        camera: None,
                        lens: None,
                        rating: None,
                        latitude: None,
                        longitude: None,
                        keywords_json: None,
                    },
                    raw_tags: vec![crate::catalog::models::RawTag {
                        name: "keyword".into(),
                        value: "trip".into(),
                    }],
                    thumb_key: Some("77.webp".into()),
                },
            },
        )
        .await
        .unwrap();
        let updated = assets.get_asset(asset.id).await.unwrap();
        assert_eq!(updated.thumb_key.as_deref(), Some("77.webp"));
        assert_eq!(updated.indexed_mtime_ns, Some(77));
    }

    #[tokio::test]
    async fn apply_index_output_fails_when_pool_closed() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let pool = catalog.pool().clone();
        pool.close().await;
        let err = apply_index_output(
            &pool,
            &IndexApplyInput {
                asset_id: 1,
                mtime_ns: 1,
                kind: "image".into(),
                prior_thumb_key: None,
                indexed: IndexOutput {
                    meta: crate::catalog::models::AssetMeta {
                        asset_id: 1,
                        capture_at: None,
                        camera: None,
                        lens: None,
                        rating: None,
                        latitude: None,
                        longitude: None,
                        keywords_json: None,
                    },
                    raw_tags: vec![],
                    thumb_key: None,
                },
            },
        )
        .await
        .unwrap_err();
        assert!(!err.to_string().is_empty());
    }

    #[tokio::test]
    async fn index_asset_and_apply_propagates_spawn_blocking_failure() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let pool = catalog.pool().clone();
        std::env::set_var("MEMHG_TEST_INDEX_BATCH_PANIC", "1");
        let err = index_asset_and_apply(
            &pool,
            1,
            &dir.path().join("missing.jpg"),
            &dir.path().join("thumbs"),
            1,
            "image",
            None,
        )
        .await
        .unwrap_err();
        assert!(err.to_string().contains("index batch panic"));
    }
}
