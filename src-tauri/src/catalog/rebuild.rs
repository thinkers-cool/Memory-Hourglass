use crate::catalog::pools::CatalogPools;
use crate::catalog::tables::CATALOG_DATA_TABLES;
use crate::error::Result;
use crate::scan::{ScanControl, ScanService};
use std::path::PathBuf;

pub async fn rebuild_and_rescan(pools: CatalogPools, thumb_dir: PathBuf) -> Result<()> {
    for table in CATALOG_DATA_TABLES {
        sqlx::query(&format!("DELETE FROM {}", table))
            .execute(pools.write())
            .await?;
    }

    let roots = sqlx::query_as::<_, (i64,)>("SELECT id FROM source_root ORDER BY id")
        .fetch_all(pools.read())
        .await?;

    let scanner = ScanService::new(pools, thumb_dir);
    let ctrl = ScanControl::noop();
    for (root_id,) in roots {
        scanner.scan_root(root_id, &ctrl).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::repo::{AssetRepo, SourceRootRepo};
    use crate::catalog::Catalog;
    use crate::scan::{ScanControl, ScanService};
    use tempfile::tempdir;

    #[tokio::test]
    async fn rebuild_and_rescan_clears_assets_and_rescans_roots() {
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        std::fs::write(
            photos.join("one.jpg"),
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();

        let db = dir.path().join("catalog.db");
        let catalog = Catalog::open(&db).await.unwrap();
        let pools = catalog.pools().clone();
        let thumb_dir = dir.path().join("thumbs");
        let root = SourceRootRepo::new(pools.clone())
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();

        ScanService::new(pools.clone(), thumb_dir.clone())
            .scan_root(root.id, &ScanControl::noop())
            .await
            .unwrap();

        let before = AssetRepo::new(pools.clone())
            .find_by_path(root.id, "one.jpg")
            .await
            .unwrap();
        assert!(before.is_some());

        sqlx::query("INSERT INTO tag (name, parent_id, color) VALUES ('travel', NULL, '#fff')")
            .execute(catalog.write_pool())
            .await
            .unwrap();

        rebuild_and_rescan(pools.clone(), thumb_dir.clone())
            .await
            .unwrap();

        let tag_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM tag")
            .fetch_one(catalog.pool())
            .await
            .unwrap();
        assert_eq!(tag_count.0, 0);

        let after = AssetRepo::new(pools.clone())
            .find_by_path(root.id, "one.jpg")
            .await
            .unwrap();
        assert!(after.is_some());
    }

    #[tokio::test]
    async fn rebuild_and_rescan_tolerates_empty_roots() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        rebuild_and_rescan(catalog.pools().clone(), dir.path().join("thumbs"))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn rebuild_and_rescan_walks_every_root() {
        let dir = tempdir().unwrap();
        let first = dir.path().join("first");
        let second = dir.path().join("second");
        std::fs::create_dir_all(&first).unwrap();
        std::fs::create_dir_all(&second).unwrap();
        std::fs::write(
            first.join("one.jpg"),
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();
        std::fs::write(
            second.join("two.jpg"),
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();

        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let pools = catalog.pools().clone();
        let thumb_dir = dir.path().join("thumbs");
        let roots = SourceRootRepo::new(pools.clone());
        let first_root = roots
            .insert_root(first.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        let second_root = roots
            .insert_root(second.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();

        rebuild_and_rescan(pools.clone(), thumb_dir).await.unwrap();

        let assets = AssetRepo::new(pools);
        assert!(assets
            .find_by_path(first_root.id, "one.jpg")
            .await
            .unwrap()
            .is_some());
        assert!(assets
            .find_by_path(second_root.id, "two.jpg")
            .await
            .unwrap()
            .is_some());
    }
}
