use crate::catalog::repo::{AssetMetaRepo, AssetRepo, SourceRootRepo, TagRepo};
use crate::error::Result;
use crate::link::LinkService;
use crate::metadata::{metadata_context_for_asset, MetadataService};
use crate::workspace::WorkspaceMediaSettings;
use super::metadata_refresh::refresh_asset_after_metadata_write;
use sqlx::SqlitePool;
use std::collections::HashSet;
use std::path::PathBuf;

pub fn parse_keywords_json(json: &str) -> Vec<String> {
    serde_json::from_str::<Vec<String>>(json).unwrap_or_default()
}

fn merge_keywords_for_xmp(
    assigned_names: Vec<String>,
    stored_keywords_json: Option<&String>,
    catalog_names: &HashSet<String>,
) -> Vec<String> {
    let mut keywords = assigned_names;
    let mut seen: HashSet<String> = keywords.iter().cloned().collect();

    if let Some(json) = stored_keywords_json {
        for keyword in parse_keywords_json(json) {
            if !catalog_names.contains(&keyword) && seen.insert(keyword.clone()) {
                keywords.push(keyword);
            }
        }
    }

    keywords.sort();
    keywords
}

pub async fn sync_asset_tag_keywords(
    pool: &SqlitePool,
    asset_id: i64,
    settings: &WorkspaceMediaSettings,
) -> Result<()> {
    let assets = AssetRepo::new(pool.clone());
    let meta_repo = AssetMetaRepo::new(pool.clone());
    let tag_repo = TagRepo::new(pool.clone());
    let roots = SourceRootRepo::new(pool.clone());

    let asset = assets.get_asset(asset_id).await?;
    let root = roots.get_root(asset.root_id).await?;
    let abs_path = PathBuf::from(&root.path).join(&asset.rel_path);

    let assigned_names = tag_repo.list_names_for_asset(asset_id).await?;
    let catalog_names: HashSet<String> = tag_repo.list_all_names().await?.into_iter().collect();
    let stored = meta_repo.get(asset_id).await?;

    let keywords = merge_keywords_for_xmp(
        assigned_names,
        stored.as_ref().and_then(|meta| meta.keywords_json.as_ref()),
        &catalog_names,
    );

    let metadata_ctx = metadata_context_for_asset(
        settings.read_only,
        &settings.workspace_xmp_dir,
        asset.root_id,
        &asset.rel_path,
        abs_path.clone(),
    );
    MetadataService::write_keywords(&metadata_ctx, &keywords)?;
    refresh_asset_after_metadata_write(
        pool,
        asset_id,
        &abs_path,
        &metadata_ctx,
        settings.read_only,
    )
    .await?;
    Ok(())
}

pub async fn sync_assets_tag_keywords(
    pool: &SqlitePool,
    asset_ids: &[i64],
    settings: &crate::workspace::WorkspaceMediaSettings,
) -> Result<()> {
    for &asset_id in asset_ids {
        sync_asset_tag_keywords(pool, asset_id, settings).await?;
    }
    if !asset_ids.is_empty() {
        LinkService::new(pool.clone())
            .refresh_duplicate_index()
            .await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::models::AssetMeta;
    use crate::catalog::repo::{AssetMetaRepo, AssetRepo, SourceRootRepo, TagRepo, UpsertAssetInput};
    use crate::catalog::Catalog;
    use crate::workspace::WorkspaceMediaSettings;
    use tempfile::tempdir;

    #[test]
    fn parse_keywords_json_invalid_returns_empty() {
        assert!(parse_keywords_json("not-json").is_empty());
    }

    #[test]
    fn parse_keywords_json_parses_array() {
        assert_eq!(
            parse_keywords_json(r#"["a","b"]"#),
            vec!["a".to_string(), "b".to_string()]
        );
    }

    #[tokio::test]
    async fn sync_asset_tag_keywords_errors_for_missing_asset() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let settings = WorkspaceMediaSettings::default();
        let err = sync_asset_tag_keywords(catalog.pool(), 999_999, &settings)
            .await
            .unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[tokio::test]
    async fn sync_assets_tag_keywords_empty_skips_work() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let settings = WorkspaceMediaSettings {
            read_only: false,
            workspace_xmp_dir: dir.path().join("xmp"),
        };
        sync_assets_tag_keywords(catalog.pool(), &[], &settings)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn sync_asset_tag_keywords_without_stored_keywords() {
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        std::fs::write(
            photos.join("plain.cr2"),
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let pool = catalog.pool().clone();
        let roots = SourceRootRepo::new(pool.clone());
        let assets = AssetRepo::new(pool.clone());
        let tags = TagRepo::new(pool.clone());
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        let asset = assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "plain.cr2",
                file_name: "plain.cr2",
                ext: "cr2",
                kind: "raw",
                size: 1,
                mtime_ns: 1,
                sync_state: "ok",
            })
            .await
            .unwrap();
        let tag_id = tags.create_tag("solo", None, None).await.unwrap();
        tags.append_tag_id_to_assets(&[asset.id], tag_id)
            .await
            .unwrap();
        let settings = WorkspaceMediaSettings {
            read_only: false,
            workspace_xmp_dir: dir.path().join("xmp"),
        };
        sync_asset_tag_keywords(&pool, asset.id, &settings)
            .await
            .unwrap();
        assert!(photos.join("plain.cr2.xmp").exists());
    }

    #[tokio::test]
    async fn sync_assets_tag_keywords_multiple_assets_refreshes_index() {
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        let jpeg = include_bytes!("../../tests/fixtures/minimal.jpg");
        std::fs::write(photos.join("one.jpg"), jpeg).unwrap();
        std::fs::write(photos.join("two.jpg"), jpeg).unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let pool = catalog.pool().clone();
        let roots = SourceRootRepo::new(pool.clone());
        let assets = AssetRepo::new(pool.clone());
        let tags = TagRepo::new(pool.clone());
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        let one = assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "one.jpg",
                file_name: "one.jpg",
                ext: "jpg",
                kind: "image",
                size: 1,
                mtime_ns: 1,
                sync_state: "ok",
            })
            .await
            .unwrap();
        let two = assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "two.jpg",
                file_name: "two.jpg",
                ext: "jpg",
                kind: "image",
                size: 1,
                mtime_ns: 2,
                sync_state: "ok",
            })
            .await
            .unwrap();
        let tag_id = tags.create_tag("batch-tag", None, None).await.unwrap();
        tags.append_tag_id_to_assets(&[one.id, two.id], tag_id)
            .await
            .unwrap();
        let settings = WorkspaceMediaSettings {
            read_only: false,
            workspace_xmp_dir: dir.path().join("xmp"),
        };
        sync_assets_tag_keywords(&pool, &[one.id, two.id], &settings)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn sync_asset_tag_keywords_skips_catalog_keyword_duplicates() {
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        std::fs::write(
            photos.join("tagged.cr2"),
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let pool = catalog.pool().clone();
        let roots = SourceRootRepo::new(pool.clone());
        let assets = AssetRepo::new(pool.clone());
        let meta_repo = AssetMetaRepo::new(pool.clone());
        let tags = TagRepo::new(pool.clone());
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        let asset = assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "tagged.cr2",
                file_name: "tagged.cr2",
                ext: "cr2",
                kind: "raw",
                size: 1,
                mtime_ns: 1,
                sync_state: "ok",
            })
            .await
            .unwrap();
        let tag_id = tags.create_tag("shared", None, None).await.unwrap();
        tags.append_tag_id_to_assets(&[asset.id], tag_id)
            .await
            .unwrap();
        meta_repo
            .upsert(&AssetMeta {
                asset_id: asset.id,
                keywords_json: Some(r#"["shared","extra"]"#.into()),
                capture_at: None,
                camera: None,
                lens: None,
                rating: None,
                latitude: None,
                longitude: None,
            })
            .await
            .unwrap();
        let settings = WorkspaceMediaSettings {
            read_only: false,
            workspace_xmp_dir: dir.path().join("xmp"),
        };
        sync_asset_tag_keywords(&pool, asset.id, &settings)
            .await
            .unwrap();
        assert!(photos.join("tagged.cr2.xmp").exists());
    }

    #[tokio::test]
    async fn sync_asset_tag_keywords_merges_orphan_stored_keywords() {
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        std::fs::write(photos.join("tagged.cr2"), b"raw").unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let pool = catalog.pool().clone();
        let roots = SourceRootRepo::new(pool.clone());
        let assets = AssetRepo::new(pool.clone());
        let meta_repo = AssetMetaRepo::new(pool.clone());
        let tags = TagRepo::new(pool.clone());
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        let asset = assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "tagged.cr2",
                file_name: "tagged.cr2",
                ext: "cr2",
                kind: "raw",
                size: 1,
                mtime_ns: 1,
                sync_state: "ok",
            })
            .await
            .unwrap();
        let tag_id = tags.create_tag("catalog-tag", None, None).await.unwrap();
        tags.append_tag_id_to_assets(&[asset.id], tag_id)
            .await
            .unwrap();
        meta_repo
            .upsert(&AssetMeta {
                asset_id: asset.id,
                keywords_json: Some(r#"["orphan"]"#.into()),
                capture_at: None,
                camera: None,
                lens: None,
                rating: None,
                latitude: None,
                longitude: None,
            })
            .await
            .unwrap();
        let settings = WorkspaceMediaSettings {
            read_only: false,
            workspace_xmp_dir: dir.path().join("xmp"),
        };
        sync_asset_tag_keywords(&pool, asset.id, &settings)
            .await
            .unwrap();
        assert!(photos.join("tagged.cr2.xmp").exists());
    }

    #[tokio::test]
    async fn sync_asset_tag_keywords_skips_duplicate_orphan_keywords() {
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        std::fs::write(
            photos.join("dup.cr2"),
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let pool = catalog.pool().clone();
        let roots = SourceRootRepo::new(pool.clone());
        let assets = AssetRepo::new(pool.clone());
        let meta_repo = AssetMetaRepo::new(pool.clone());
        let tags = TagRepo::new(pool.clone());
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        let asset = assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "dup.cr2",
                file_name: "dup.cr2",
                ext: "cr2",
                kind: "raw",
                size: 1,
                mtime_ns: 1,
                sync_state: "ok",
            })
            .await
            .unwrap();
        let tag_id = tags.create_tag("solo", None, None).await.unwrap();
        tags.append_tag_id_to_assets(&[asset.id], tag_id)
            .await
            .unwrap();
        meta_repo
            .upsert(&AssetMeta {
                asset_id: asset.id,
                keywords_json: Some(r#"["solo","orphan","orphan"]"#.into()),
                capture_at: None,
                camera: None,
                lens: None,
                rating: None,
                latitude: None,
                longitude: None,
            })
            .await
            .unwrap();
        let settings = WorkspaceMediaSettings {
            read_only: false,
            workspace_xmp_dir: dir.path().join("xmp"),
        };
        sync_asset_tag_keywords(&pool, asset.id, &settings)
            .await
            .unwrap();
        assert!(photos.join("dup.cr2.xmp").exists());
    }

    #[tokio::test]
    async fn sync_asset_tag_keywords_errors_after_pool_close() {
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        std::fs::write(
            photos.join("close.cr2"),
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let pool = catalog.pool().clone();
        let roots = SourceRootRepo::new(pool.clone());
        let assets = AssetRepo::new(pool.clone());
        let tags = TagRepo::new(pool.clone());
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        let asset = assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "close.cr2",
                file_name: "close.cr2",
                ext: "cr2",
                kind: "raw",
                size: 1,
                mtime_ns: 1,
                sync_state: "ok",
            })
            .await
            .unwrap();
        let tag_id = tags.create_tag("close", None, None).await.unwrap();
        tags.append_tag_id_to_assets(&[asset.id], tag_id)
            .await
            .unwrap();
        let settings = WorkspaceMediaSettings {
            read_only: false,
            workspace_xmp_dir: dir.path().join("xmp"),
        };
        pool.close().await;
        assert!(
            sync_asset_tag_keywords(&pool, asset.id, &settings)
                .await
                .is_err()
        );
        assert!(
            sync_assets_tag_keywords(&pool, &[asset.id], &settings)
                .await
                .is_err()
        );
    }
}
