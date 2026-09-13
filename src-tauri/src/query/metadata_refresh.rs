use crate::catalog::models::AssetMeta;
use crate::catalog::repo::{AssetMetaRepo, AssetRepo, RawTagRepo};
use crate::error::{AppError, Result};
use crate::link::LinkService;
use crate::metadata::{MetadataContext, MetadataService};
use crate::scan::file_mtime_ns;
use crate::catalog::pools::CatalogPools;
use std::path::Path;

pub async fn refresh_asset_after_metadata_write(
    pools: &CatalogPools,
    asset_id: i64,
    abs_path: &Path,
    metadata_ctx: &MetadataContext,
    read_only: bool,
) -> Result<()> {
    let assets = AssetRepo::new(pools.clone());
    let meta_repo = AssetMetaRepo::new(pools.clone());
    let raw_tag_repo = RawTagRepo::new(pools.clone());
    let link = LinkService::new(pools.clone());

    let file_meta = std::fs::metadata(abs_path)?;
    let mtime_ns = file_mtime_ns(&file_meta)
        .ok_or_else(|| AppError::Metadata(format!("missing mtime for {}", abs_path.display())))?;
    let size = file_meta.len() as i64;

    if !read_only {
        assets.update_file_stats(asset_id, mtime_ns, size).await?;
    }

    let ctx = metadata_ctx.clone();
    let read_meta_panic = crate::scan::test_hooks::take_flag("MEMHG_TEST_READ_META_PANIC");
    let (read_meta, raw_tags) =
        tokio::task::spawn_blocking(move || {
            if read_meta_panic {
                panic!("read meta panic");
            }
            MetadataService::read_meta(&ctx)
        })
            .await
            .map_err(|error| AppError::Metadata(error.to_string()))??;

    meta_repo
        .upsert(&AssetMeta {
            asset_id,
            capture_at: read_meta.capture_at,
            camera: read_meta.camera,
            lens: read_meta.lens,
            rating: read_meta.rating,
            latitude: read_meta.latitude,
            longitude: read_meta.longitude,
            keywords_json: read_meta.keywords_json,
            rotation: read_meta.rotation,
        })
        .await?;
    raw_tag_repo.replace_for_asset(asset_id, &raw_tags).await?;
    assets.mark_indexed(asset_id, mtime_ns).await?;
    if !read_only {
        link.recompute_content_hash(asset_id, abs_path).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::repo::{AssetMetaRepo, AssetRepo, SourceRootRepo};
    use crate::catalog::Catalog;
    use crate::metadata::MetadataContext;
    use crate::metadata::MetadataService;
    use crate::scan::{ScanControl, ScanService};
    use tempfile::tempdir;

    #[tokio::test]
    async fn refresh_asset_after_metadata_write_updates_catalog() {
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        let file_path = photos.join("rated.jpg");
        std::fs::write(
            &file_path,
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();

        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let pools = catalog.pools().clone();
        let thumb_dir = dir.path().join("thumbs");
        let roots = SourceRootRepo::new(pools.clone());
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        ScanService::new(pools.clone(), thumb_dir)
            .scan_root(root.id, &ScanControl::noop())
            .await
            .unwrap();

        let asset_id: i64 = sqlx::query_scalar("SELECT id FROM asset WHERE root_id = ? LIMIT 1")
            .bind(root.id)
            .fetch_one(pools.read())
            .await
            .unwrap();
        let ctx = MetadataContext::in_place(file_path.clone());
        MetadataService::write_rating(&ctx, 4).unwrap();

        refresh_asset_after_metadata_write(&pools, asset_id, &file_path, &ctx, false)
            .await
            .unwrap();

        let meta = AssetMetaRepo::new(pools.clone())
            .get(asset_id)
            .await
            .unwrap()
            .expect("meta");
        assert_eq!(meta.rating, Some(4));

        let tags =
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM asset_raw_tag WHERE asset_id = ?")
                .bind(asset_id)
                .fetch_one(pools.read())
                .await
                .unwrap();
        assert!(tags > 0);

        let hash: Option<String> =
            sqlx::query_scalar("SELECT content_hash FROM asset WHERE id = ?")
                .bind(asset_id)
                .fetch_one(pools.read())
                .await
                .unwrap();
        assert!(hash.is_some());

        let assets = AssetRepo::new(pools.clone());
        let asset = assets.get_asset(asset_id).await.unwrap();
        assert_eq!(asset.sync_state, "ok");
    }

    #[tokio::test]
    async fn refresh_asset_skips_hash_in_read_only_mode() {
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        let file_path = photos.join("readonly.jpg");
        std::fs::write(
            &file_path,
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();

        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let pools = catalog.pools().clone();
        let thumb_dir = dir.path().join("thumbs");
        let roots = SourceRootRepo::new(pools.clone());
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        ScanService::new(pools.clone(), thumb_dir)
            .scan_root(root.id, &ScanControl::noop())
            .await
            .unwrap();

        let asset_id: i64 = sqlx::query_scalar("SELECT id FROM asset WHERE root_id = ? LIMIT 1")
            .bind(root.id)
            .fetch_one(pools.read())
            .await
            .unwrap();
        sqlx::query("UPDATE asset SET content_hash = ? WHERE id = ?")
            .bind("seed-hash")
            .bind(asset_id)
            .execute(pools.write())
            .await
            .unwrap();

        let ctx = MetadataContext::in_place(file_path.clone());
        MetadataService::write_rating(&ctx, 2).unwrap();

        refresh_asset_after_metadata_write(&pools, asset_id, &file_path, &ctx, true)
            .await
            .unwrap();

        let hash: String = sqlx::query_scalar("SELECT content_hash FROM asset WHERE id = ?")
            .bind(asset_id)
            .fetch_one(pools.read())
            .await
            .unwrap();
        assert_eq!(hash, "seed-hash");
    }

    #[tokio::test]
    async fn refresh_asset_errors_when_file_missing() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let pools = catalog.pools().clone();
        let ctx = MetadataContext::in_place(dir.path().join("missing.jpg"));
        let err = refresh_asset_after_metadata_write(
            &pools,
            1,
            &dir.path().join("missing.jpg"),
            &ctx,
            false,
        )
        .await
        .unwrap_err();
        assert!(!err.to_string().is_empty());
    }

    #[tokio::test]
    async fn refresh_asset_updates_file_stats_in_read_write_mode() {
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        let file_path = photos.join("stats.jpg");
        std::fs::write(
            &file_path,
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();

        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let pools = catalog.pools().clone();
        let thumb_dir = dir.path().join("thumbs");
        let roots = SourceRootRepo::new(pools.clone());
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        ScanService::new(pools.clone(), thumb_dir)
            .scan_root(root.id, &ScanControl::noop())
            .await
            .unwrap();

        let asset_id: i64 = sqlx::query_scalar("SELECT id FROM asset WHERE root_id = ? LIMIT 1")
            .bind(root.id)
            .fetch_one(pools.read())
            .await
            .unwrap();
        let before_size: i64 = sqlx::query_scalar("SELECT size FROM asset WHERE id = ?")
            .bind(asset_id)
            .fetch_one(pools.read())
            .await
            .unwrap();
        std::fs::write(
            &file_path,
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();
        let ctx = MetadataContext::in_place(file_path.clone());
        MetadataService::write_rating(&ctx, 1).unwrap();

        refresh_asset_after_metadata_write(&pools, asset_id, &file_path, &ctx, false)
            .await
            .unwrap();

        let after_size: i64 = sqlx::query_scalar("SELECT size FROM asset WHERE id = ?")
            .bind(asset_id)
            .fetch_one(pools.read())
            .await
            .unwrap();
        assert!(after_size >= before_size);
        let raw_tags: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM asset_raw_tag WHERE asset_id = ?")
                .bind(asset_id)
                .fetch_one(pools.read())
                .await
                .unwrap();
        assert!(raw_tags > 0);
    }

    #[tokio::test]
    async fn refresh_asset_errors_when_mtime_missing() {
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        let file_path = photos.join("mtime.jpg");
        std::fs::write(
            &file_path,
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();

        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let pools = catalog.pools().clone();
        let ctx = MetadataContext::in_place(file_path.clone());
        crate::scan::test_hooks::set_flag("MEMHG_TEST_NULL_MTIME");
        let err = refresh_asset_after_metadata_write(&pools, 1, &file_path, &ctx, false)
            .await
            .unwrap_err();
        assert!(err.to_string().contains("mtime"));
        crate::scan::test_hooks::clear_flag("MEMHG_TEST_NULL_MTIME");
    }

    #[tokio::test]
    async fn refresh_asset_propagates_spawn_blocking_panic() {
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        let file_path = photos.join("panic.jpg");
        std::fs::write(
            &file_path,
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let pools = catalog.pools().clone();
        let thumb_dir = dir.path().join("thumbs");
        let roots = SourceRootRepo::new(pools.clone());
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        ScanService::new(pools.clone(), thumb_dir)
            .scan_root(root.id, &ScanControl::noop())
            .await
            .unwrap();
        let asset_id: i64 = sqlx::query_scalar("SELECT id FROM asset WHERE root_id = ? LIMIT 1")
            .bind(root.id)
            .fetch_one(pools.read())
            .await
            .unwrap();
        let ctx = MetadataContext::in_place(file_path.clone());
        crate::scan::test_hooks::set_flag("MEMHG_TEST_READ_META_PANIC");
        let err = refresh_asset_after_metadata_write(&pools, asset_id, &file_path, &ctx, false)
            .await
            .unwrap_err();
        assert!(err.to_string().contains("read meta panic"));
    }

    #[tokio::test]
    async fn refresh_asset_errors_after_pool_close() {
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        let file_path = photos.join("pool.jpg");
        std::fs::write(
            &file_path,
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();

        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let pools = catalog.pools().clone();
        let thumb_dir = dir.path().join("thumbs");
        let roots = SourceRootRepo::new(pools.clone());
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        ScanService::new(pools.clone(), thumb_dir)
            .scan_root(root.id, &ScanControl::noop())
            .await
            .unwrap();

        let asset_id: i64 = sqlx::query_scalar("SELECT id FROM asset WHERE root_id = ? LIMIT 1")
            .bind(root.id)
            .fetch_one(pools.read())
            .await
            .unwrap();
        let ctx = MetadataContext::in_place(file_path.clone());
        MetadataService::write_rating(&ctx, 3).unwrap();
        pools.close().await;
        assert!(
            refresh_asset_after_metadata_write(&pools, asset_id, &file_path, &ctx, false)
                .await
                .is_err()
        );
    }
}
