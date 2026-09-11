use super::AssetRepo;
use crate::error::Result;

#[derive(sqlx::FromRow)]
struct PurgeRow {
    id: i64,
    rel_path: String,
    root_path: String,
}

pub(crate) fn remove_purged_asset_files(root_path: &str, rel_path: &str) -> Result<()> {
    let path = crate::library::resolve_path_under_root(std::path::Path::new(root_path), rel_path)?;
    if path.exists() {
        std::fs::remove_file(&path)?;
    }
    let sidecar = crate::metadata::xmp_sidecar_path(&path);
    if sidecar.exists() {
        std::fs::remove_file(&sidecar)?;
    }
    Ok(())
}

impl AssetRepo {
    pub async fn purge_assets(
        &self,
        ids: &[i64],
        _workspace_paths: &crate::workspace::WorkspacePaths,
        read_only: bool,
    ) -> Result<Vec<i64>> {
        if read_only {
            return Err(crate::error::AppError::InvalidInput(
                "cannot purge source files in a read-only workspace".into(),
            ));
        }
        let mut targets = Vec::new();
        for id in ids {
            let row = sqlx::query_as::<_, PurgeRow>(
                r#"
                SELECT a.id, a.rel_path, r.path as root_path
                FROM asset a
                JOIN source_root r ON r.id = a.root_id
                WHERE a.id = ?
                "#,
            )
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
            if let Some(row) = row {
                targets.push(row);
            }
        }

        if targets.is_empty() {
            return Ok(Vec::new());
        }

        let mut tx = self.pool.begin().await?;
        for row in &targets {
            sqlx::query("DELETE FROM asset WHERE id = ?")
                .bind(row.id)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;

        let mut purged_ids = Vec::with_capacity(targets.len());
        for row in targets {
            remove_purged_asset_files(&row.root_path, &row.rel_path)?;
            purged_ids.push(row.id);
        }
        Ok(purged_ids)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::repo::test_support::test_catalog;
    use crate::catalog::repo::{AssetRepo, SourceRootRepo, UpsertAssetInput};

    #[test]
    fn remove_purged_asset_files_skips_missing_media() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("photos");
        std::fs::create_dir_all(&root).unwrap();
        remove_purged_asset_files(root.to_str().unwrap(), "missing.jpg").unwrap();
    }

    #[test]
    fn remove_purged_asset_files_deletes_media_and_sidecar() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("photos");
        std::fs::create_dir_all(&root).unwrap();
        let image = root.join("purge.jpg");
        std::fs::write(
            &image,
            include_bytes!("../../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();
        let sidecar = crate::metadata::xmp_sidecar_path(&image);
        std::fs::write(&sidecar, b"xmp").unwrap();
        remove_purged_asset_files(root.to_str().unwrap(), "purge.jpg").unwrap();
        assert!(!image.exists());
        assert!(!sidecar.exists());
    }

    #[test]
    fn remove_purged_asset_files_errors_when_media_path_is_directory() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("photos");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(root.join("blocked.jpg")).unwrap();
        assert!(remove_purged_asset_files(root.to_str().unwrap(), "blocked.jpg").is_err());
    }

    #[test]
    fn remove_purged_asset_files_errors_when_sidecar_is_directory() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("photos");
        std::fs::create_dir_all(&root).unwrap();
        let image = root.join("shot.jpg");
        std::fs::write(
            &image,
            include_bytes!("../../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();
        let sidecar = crate::metadata::xmp_sidecar_path(&image);
        std::fs::create_dir_all(&sidecar).unwrap();
        assert!(remove_purged_asset_files(root.to_str().unwrap(), "shot.jpg").is_err());
    }

    #[tokio::test]
    async fn purge_assets_rejects_read_only_workspace() {
        let (catalog, dir) = test_catalog().await;
        let assets = AssetRepo::new(catalog.pool().clone());
        let paths = crate::workspace::WorkspacePaths::new(dir.path().to_path_buf());
        let err = assets.purge_assets(&[1], &paths, true).await.unwrap_err();
        assert!(err.to_string().contains("read-only"));
    }

    #[tokio::test]
    async fn purge_assets_removes_file_and_sidecar() {
        let (catalog, dir) = test_catalog().await;
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let assets = AssetRepo::new(catalog.pool().clone());
        let photos = std::fs::canonicalize({
            let photos = dir.path().join("photos");
            std::fs::create_dir_all(&photos).unwrap();
            photos
        })
        .unwrap();
        let image_path = photos.join("purge.jpg");
        std::fs::write(
            &image_path,
            include_bytes!("../../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();
        let sidecar = crate::metadata::xmp_sidecar_path(&image_path);
        std::fs::write(&sidecar, b"xmp").unwrap();
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        let asset = assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "purge.jpg",
                file_name: "purge.jpg",
                ext: "jpg",
                kind: "image",
                size: 1,
                mtime_ns: 1,
                sync_state: "ok",
            })
            .await
            .unwrap();
        let paths = crate::workspace::WorkspacePaths::new(dir.path().to_path_buf());
        let root_record = roots.get_root(root.id).await.unwrap();
        let disk_path = std::path::PathBuf::from(&root_record.path).join("purge.jpg");
        assert!(disk_path.exists());
        assert!(sidecar.exists());
        assets
            .purge_assets(&[asset.id], &paths, false)
            .await
            .unwrap();
        assert!(!disk_path.exists());
        assert!(!sidecar.exists());
        let err = assets.get_asset(asset.id).await.unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[tokio::test]
    async fn purge_assets_rejects_traversal_rel_path() {
        let (catalog, dir) = test_catalog().await;
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let assets = AssetRepo::new(catalog.pool().clone());
        let paths = crate::workspace::WorkspacePaths::new(dir.path().to_path_buf());
        let photos = std::fs::canonicalize({
            let photos = dir.path().join("purge-traversal");
            std::fs::create_dir_all(&photos).unwrap();
            photos
        })
        .unwrap();
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        let outside = dir.path().join("outside-secret.jpg");
        std::fs::write(&outside, b"secret").unwrap();
        let asset = assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "../outside-secret.jpg",
                file_name: "outside-secret.jpg",
                ext: "jpg",
                kind: "image",
                size: 1,
                mtime_ns: 1,
                sync_state: "ok",
            })
            .await
            .unwrap();
        let err = assets
            .purge_assets(&[asset.id], &paths, false)
            .await
            .unwrap_err();
        assert!(outside.exists());
        assert!(!err.to_string().is_empty());
    }

    #[tokio::test]
    async fn purge_assets_noops_missing_files_for_unknown_id() {
        let (catalog, dir) = test_catalog().await;
        let assets = AssetRepo::new(catalog.pool().clone());
        let paths = crate::workspace::WorkspacePaths::new(dir.path().to_path_buf());
        let deleted = assets
            .purge_assets(&[999_999], &paths, false)
            .await
            .unwrap();
        assert!(deleted.is_empty());
    }

    #[tokio::test]
    async fn purge_assets_removes_file_without_sidecar() {
        let (catalog, dir) = test_catalog().await;
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let assets = AssetRepo::new(catalog.pool().clone());
        let photos = std::fs::canonicalize({
            let photos = dir.path().join("purge-no-xmp");
            std::fs::create_dir_all(&photos).unwrap();
            photos
        })
        .unwrap();
        let image_path = photos.join("only.jpg");
        std::fs::write(
            &image_path,
            include_bytes!("../../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        let asset = assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "only.jpg",
                file_name: "only.jpg",
                ext: "jpg",
                kind: "image",
                size: 1,
                mtime_ns: 1,
                sync_state: "ok",
            })
            .await
            .unwrap();
        let paths = crate::workspace::WorkspacePaths::new(dir.path().to_path_buf());
        assets
            .purge_assets(&[asset.id], &paths, false)
            .await
            .unwrap();
        assert!(!image_path.exists());
    }

    #[tokio::test]
    async fn soft_delete_restore_and_purge_empty_helpers() {
        let (catalog, dir) = test_catalog().await;
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let assets = AssetRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root("/tmp/p", "local", "watch", None)
            .await
            .unwrap();
        let asset = assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "x.jpg",
                file_name: "x.jpg",
                ext: "jpg",
                kind: "image",
                size: 1,
                mtime_ns: 1,
                sync_state: "ok",
            })
            .await
            .unwrap();
        assert_eq!(assets.soft_delete(&[asset.id], 1).await.unwrap(), 1);
        assert_eq!(assets.soft_delete(&[asset.id], 2).await.unwrap(), 0);
        assert_eq!(assets.restore_assets(&[asset.id]).await.unwrap(), 1);
        assert_eq!(assets.restore_assets(&[asset.id]).await.unwrap(), 0);
        let paths = crate::workspace::WorkspacePaths::new(dir.path().to_path_buf());
        assert!(assets
            .purge_assets(&[], &paths, false)
            .await
            .unwrap()
            .is_empty());
    }

    #[tokio::test]
    async fn purge_assets_errors_under_exclusive_lock_after_file_removed() {
        let (catalog, dir) = test_catalog().await;
        let pool = catalog.pool().clone();
        let roots = SourceRootRepo::new(pool.clone());
        let assets = AssetRepo::new(pool.clone());
        let photos = std::fs::canonicalize({
            let photos = dir.path().join("purge-lock");
            std::fs::create_dir_all(&photos).unwrap();
            photos
        })
        .unwrap();
        let image_path = photos.join("gone.jpg");
        std::fs::write(
            &image_path,
            include_bytes!("../../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        let asset = assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "gone.jpg",
                file_name: "gone.jpg",
                ext: "jpg",
                kind: "image",
                size: 1,
                mtime_ns: 1,
                sync_state: "ok",
            })
            .await
            .unwrap();
        let paths = crate::workspace::WorkspacePaths::new(dir.path().to_path_buf());
        let mut locker = pool.acquire().await.unwrap();
        sqlx::query("BEGIN EXCLUSIVE")
            .execute(&mut *locker)
            .await
            .unwrap();
        assert!(assets
            .purge_assets(&[asset.id], &paths, false)
            .await
            .is_err());
        assert!(image_path.exists());
        sqlx::query("ROLLBACK").execute(&mut *locker).await.unwrap();
    }

    #[tokio::test]
    async fn purge_assets_propagates_filesystem_errors() {
        let (catalog, dir) = test_catalog().await;
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let assets = AssetRepo::new(catalog.pool().clone());
        let photos = std::fs::canonicalize({
            let photos = dir.path().join("purge-fs");
            std::fs::create_dir_all(&photos).unwrap();
            photos
        })
        .unwrap();
        std::fs::create_dir_all(photos.join("blocked.jpg")).unwrap();
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        let asset = assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "blocked.jpg",
                file_name: "blocked.jpg",
                ext: "jpg",
                kind: "image",
                size: 1,
                mtime_ns: 1,
                sync_state: "ok",
            })
            .await
            .unwrap();
        let paths = crate::workspace::WorkspacePaths::new(dir.path().to_path_buf());
        assert!(assets
            .purge_assets(&[asset.id], &paths, false)
            .await
            .is_err());
    }

    #[tokio::test]
    async fn asset_repo_purge_errors_after_pool_close() {
        let (catalog, dir) = test_catalog().await;
        let pool = catalog.pool().clone();
        let roots = SourceRootRepo::new(pool.clone());
        let assets = AssetRepo::new(pool.clone());
        let root = roots
            .insert_root("/tmp/p", "local", "watch", None)
            .await
            .unwrap();
        let asset = assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "a.jpg",
                file_name: "a.jpg",
                ext: "jpg",
                kind: "image",
                size: 1,
                mtime_ns: 1,
                sync_state: "ok",
            })
            .await
            .unwrap();
        let paths = crate::workspace::WorkspacePaths::new(dir.path().to_path_buf());
        pool.close().await;
        assert!(assets
            .purge_assets(&[asset.id], &paths, false)
            .await
            .is_err());
    }
}
