use crate::catalog::models::{DuplicateAsset, LinkedAsset};
use crate::catalog::pools::CatalogPools;
use crate::error::{AppError, Result};
use std::collections::HashMap;
use std::path::Path;

pub struct LinkService {
    pools: CatalogPools,
}

impl LinkService {
    pub fn new(pools: CatalogPools) -> Self {
        Self { pools }
    }

    pub async fn link_raw_jpeg_in_root(&self, root_id: i64) -> Result<u64> {
        let rows = sqlx::query_as::<_, AssetRow>(
            "SELECT id, file_name, ext, kind, rel_path FROM asset WHERE root_id = ? AND deleted_at IS NULL",
        )
        .bind(root_id)
        .fetch_all(self.pools.read())
        .await?;

        let mut by_stem: HashMap<String, Vec<&AssetRow>> = HashMap::new();
        for row in &rows {
            let stem = stem_name(&row.file_name);
            by_stem.entry(stem).or_default().push(row);
        }

        let mut linked = 0u64;
        for group in by_stem.values() {
            let mut raws: Vec<&AssetRow> =
                group.iter().filter(|a| a.kind == "raw").copied().collect();
            let mut jpegs: Vec<&AssetRow> = group
                .iter()
                .filter(|a| {
                    let e = a.ext.to_lowercase();
                    e == "jpg" || e == "jpeg"
                })
                .copied()
                .collect();
            raws.sort_by(|a, b| a.rel_path.cmp(&b.rel_path).then(a.id.cmp(&b.id)));
            jpegs.sort_by(|a, b| a.rel_path.cmp(&b.rel_path).then(a.id.cmp(&b.id)));
            if let Some(jpeg) = jpegs.first() {
                linked += self.link_raw_group(&raws, jpeg).await?;
            }
        }
        Ok(linked)
    }

    pub async fn find_duplicates_by_hash(&self, root_id: Option<i64>) -> Result<Vec<Vec<i64>>> {
        let rows = if let Some(id) = root_id {
            sqlx::query_as::<_, HashRow>(
                "SELECT id, content_hash, rel_path FROM asset WHERE root_id = ? AND content_hash IS NOT NULL AND deleted_at IS NULL",
            )
            .bind(id)
            .fetch_all(self.pools.read())
            .await?
        } else {
            sqlx::query_as::<_, HashRow>(
                "SELECT id, content_hash, rel_path FROM asset WHERE content_hash IS NOT NULL AND deleted_at IS NULL",
            )
            .fetch_all(self.pools.read())
            .await?
        };

        let mut groups: HashMap<String, Vec<HashRow>> = HashMap::new();
        for row in rows {
            if let Some(hash) = row.content_hash.clone() {
                groups.entry(hash).or_default().push(row);
            }
        }
        Ok(groups
            .into_values()
            .filter(|g| g.len() > 1)
            .map(|mut group| {
                group.sort_by(|a, b| a.rel_path.cmp(&b.rel_path).then(a.id.cmp(&b.id)));
                group.into_iter().map(|row| row.id).collect()
            })
            .collect())
    }

    pub async fn rebuild_duplicate_links(&self) -> Result<()> {
        sqlx::query("DELETE FROM asset_link WHERE kind = 'duplicate_hash'")
            .execute(self.pools.write())
            .await?;

        let groups = self.find_duplicates_by_hash(None).await?;
        for group in groups {
            let primary = group[0];
            for dup_id in group.iter().skip(1) {
                self.insert_link(primary, *dup_id, "duplicate_hash").await?;
            }
        }
        Ok(())
    }

    pub async fn refresh_duplicate_flags(&self) -> Result<()> {
        sqlx::query("UPDATE asset SET has_duplicate = 0")
            .execute(self.pools.write())
            .await?;

        sqlx::query(
            r#"
            UPDATE asset SET has_duplicate = 1
            WHERE deleted_at IS NULL
              AND content_hash IS NOT NULL
              AND content_hash IN (
                SELECT content_hash FROM asset
                WHERE content_hash IS NOT NULL AND deleted_at IS NULL
                GROUP BY content_hash HAVING COUNT(*) > 1
              )
            "#,
        )
        .execute(self.pools.write())
        .await?;

        Ok(())
    }

    pub async fn refresh_duplicate_flags_for_root(&self, root_id: i64) -> Result<()> {
        sqlx::query("UPDATE asset SET has_duplicate = 0 WHERE root_id = ?")
            .bind(root_id)
            .execute(self.pools.write())
            .await?;

        sqlx::query(
            r#"
            UPDATE asset SET has_duplicate = 1
            WHERE root_id = ?
              AND deleted_at IS NULL
              AND content_hash IS NOT NULL
              AND content_hash IN (
                SELECT content_hash FROM asset
                WHERE root_id = ?
                  AND content_hash IS NOT NULL AND deleted_at IS NULL
                GROUP BY content_hash HAVING COUNT(*) > 1
              )
            "#,
        )
        .bind(root_id)
        .bind(root_id)
        .execute(self.pools.write())
        .await?;

        Ok(())
    }

    pub async fn list_duplicates_for_asset(&self, asset_id: i64) -> Result<Vec<DuplicateAsset>> {
        Ok(sqlx::query_as::<_, DuplicateAsset>(
            r#"
            SELECT peer.id, peer.file_name, r.path as root_path, peer.rel_path
            FROM asset current
            JOIN asset peer ON peer.content_hash = current.content_hash
            JOIN source_root r ON r.id = peer.root_id
            WHERE current.id = ?
              AND peer.id != ?
              AND peer.deleted_at IS NULL
              AND current.content_hash IS NOT NULL
            ORDER BY r.path, peer.rel_path
            "#,
        )
        .bind(asset_id)
        .bind(asset_id)
        .fetch_all(self.pools.read())
        .await?)
    }

    pub async fn link_duplicates_in_root(&self, root_id: i64) -> Result<u64> {
        let groups = self.find_duplicates_by_hash(Some(root_id)).await?;
        let mut linked = 0u64;
        for group in groups {
            let primary = group[0];
            for dup_id in group.iter().skip(1) {
                self.insert_link(primary, *dup_id, "duplicate_hash").await?;
                linked += 1;
            }
        }
        Ok(linked)
    }

    pub async fn list_links(&self, asset_id: i64) -> Result<Vec<LinkedAsset>> {
        Ok(sqlx::query_as::<_, LinkedAsset>(
            r#"
            SELECT al.kind, a.id, a.file_name, a.kind as asset_kind, r.path as root_path, a.rel_path
            FROM asset_link al
            JOIN asset a ON a.id = al.dst_id
            JOIN source_root r ON r.id = a.root_id
            WHERE al.src_id = ?
              AND al.kind != 'duplicate_hash'
            UNION
            SELECT al.kind, a.id, a.file_name, a.kind as asset_kind, r.path as root_path, a.rel_path
            FROM asset_link al
            JOIN asset a ON a.id = al.src_id
            JOIN source_root r ON r.id = a.root_id
            WHERE al.dst_id = ?
              AND al.kind != 'duplicate_hash'
            "#,
        )
        .bind(asset_id)
        .bind(asset_id)
        .fetch_all(self.pools.read())
        .await?)
    }

    pub async fn recompute_content_hash(&self, asset_id: i64, path: &Path) -> Result<String> {
        let path = path.to_path_buf();
        let hash = tokio::task::spawn_blocking(move || file_sha256(&path))
            .await
            .map_err(|error| AppError::Library(format!("hash computation failed: {}", error)))??;

        sqlx::query("UPDATE asset SET content_hash = ? WHERE id = ? AND deleted_at IS NULL")
            .bind(&hash)
            .bind(asset_id)
            .execute(self.pools.write())
            .await?;

        Ok(hash)
    }

    pub async fn refresh_duplicate_index(&self) -> Result<()> {
        self.rebuild_duplicate_links().await?;
        self.refresh_duplicate_flags().await?;
        Ok(())
    }

    pub async fn compute_hashes_for_root(&self, root_id: i64, root_path: &Path) -> Result<u64> {
        let rows = sqlx::query_as::<_, PathRow>(
            r#"
            SELECT id, rel_path FROM asset
            WHERE root_id = ? AND deleted_at IS NULL
              AND (content_hash IS NULL OR sync_state IN ('new', 'modified'))
            "#,
        )
        .bind(root_id)
        .fetch_all(self.pools.read())
        .await?;

        if rows.is_empty() {
            return Ok(0);
        }

        let root_path = root_path.to_path_buf();
        let updates = tokio::task::spawn_blocking(move || {
            rows.into_iter()
                .filter_map(|row| {
                    let path = root_path.join(&row.rel_path);
                    file_sha256(&path).ok().map(|hash| (hash, row.id))
                })
                .collect::<Vec<_>>()
        })
        .await
        .map_err(|error| AppError::Library(format!("hash computation failed: {}", error)))?;

        if updates.is_empty() {
            return Ok(0);
        }

        let mut builder = sqlx::QueryBuilder::new("UPDATE asset SET content_hash = CASE id ");
        for (hash, id) in &updates {
            builder.push("WHEN ");
            builder.push_bind(*id);
            builder.push(" THEN ");
            builder.push_bind(hash);
            builder.push(" ");
        }
        builder.push("ELSE content_hash END WHERE id IN (");
        for (index, (_, id)) in updates.iter().enumerate() {
            if index > 0 {
                builder.push(", ");
            }
            builder.push_bind(*id);
        }
        builder.push(")");
        builder.build().execute(self.pools.write()).await?;
        Ok(updates.len() as u64)
    }

    async fn link_raw_group(&self, raws: &[&AssetRow], jpeg: &AssetRow) -> Result<u64> {
        let mut linked = 0u64;
        for raw in raws {
            self.insert_link(raw.id, jpeg.id, "raw_jpeg").await?;
            linked += 1;
        }
        Ok(linked)
    }

    async fn insert_link(&self, src: i64, dst: i64, kind: &str) -> Result<()> {
        sqlx::query("INSERT OR IGNORE INTO asset_link (src_id, dst_id, kind) VALUES (?, ?, ?)")
            .bind(src)
            .bind(dst)
            .bind(kind)
            .execute(self.pools.write())
            .await?;
        Ok(())
    }
}

fn stem_name(file_name: &str) -> String {
    let stem = crate::path_util::os_file_stem(Path::new(file_name));
    if stem.is_empty() {
        file_name.to_lowercase()
    } else {
        stem.to_lowercase()
    }
}

pub fn sha256_bytes(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(bytes))
}

fn file_sha256(path: &std::path::Path) -> Result<String> {
    Ok(sha256_bytes(&std::fs::read(path)?))
}

#[derive(sqlx::FromRow)]
struct AssetRow {
    id: i64,
    file_name: String,
    ext: String,
    kind: String,
    rel_path: String,
}

#[derive(sqlx::FromRow)]
struct HashRow {
    id: i64,
    content_hash: Option<String>,
    rel_path: String,
}

#[derive(sqlx::FromRow)]
struct PathRow {
    id: i64,
    rel_path: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::repo::SourceRootRepo;
    use crate::catalog::Catalog;
    use crate::scan::{ScanControl, ScanService};
    use tempfile::tempdir;

    async fn test_catalog() -> (Catalog, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        (catalog, dir)
    }

    #[tokio::test]
    async fn links_raw_and_jpeg_from_catalog_rows() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let pools = catalog.pools().clone();
        let roots = SourceRootRepo::new(pools.clone());
        let root = roots
            .insert_root(dir.path().to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        let raw_id: i64 = sqlx::query_scalar(
            "INSERT INTO asset (root_id, rel_path, file_name, ext, kind, size, mtime_ns, sync_state) VALUES (?, 'a.arw', 'a.arw', 'arw', 'raw', 1, 1, 'ok') RETURNING id",
        )
        .bind(root.id)
        .fetch_one(pools.write())
        .await
        .unwrap();
        sqlx::query_scalar::<_, i64>(
            "INSERT INTO asset (root_id, rel_path, file_name, ext, kind, size, mtime_ns, sync_state) VALUES (?, 'a.jpg', 'a.jpg', 'jpg', 'image', 1, 1, 'ok') RETURNING id",
        )
        .bind(root.id)
        .fetch_one(pools.write())
        .await
        .unwrap();
        let link = LinkService::new(pools);
        let count = link.link_raw_jpeg_in_root(root.id).await.unwrap();
        assert_eq!(count, 1);
        let links = link.list_links(raw_id).await.unwrap();
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].kind, "raw_jpeg");
    }

    #[tokio::test]
    async fn links_multiple_raws_to_first_jpeg() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let pools = catalog.pools().clone();
        let roots = SourceRootRepo::new(pools.clone());
        let root = roots
            .insert_root(dir.path().to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        for (rel_path, kind, ext) in [
            ("a.arw", "raw", "arw"),
            ("a.dng", "raw", "dng"),
            ("a.jpg", "image", "jpg"),
        ] {
            sqlx::query(
                "INSERT INTO asset (root_id, rel_path, file_name, ext, kind, size, mtime_ns, sync_state) VALUES (?, ?, ?, ?, ?, 1, 1, 'ok')",
            )
            .bind(root.id)
            .bind(rel_path)
            .bind(rel_path)
            .bind(ext)
            .bind(kind)
            .execute(pools.write())
            .await
            .unwrap();
        }
        let link = LinkService::new(pools);
        let count = link.link_raw_jpeg_in_root(root.id).await.unwrap();
        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn links_raw_and_jpeg_by_stem() {
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        let jpeg = include_bytes!("../../tests/fixtures/minimal.jpg");
        std::fs::write(photos.join("DSC001.jpg"), jpeg).unwrap();
        std::fs::write(photos.join("DSC001.arw"), jpeg).unwrap();

        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let thumb_dir = dir.path().join("thumbs");
        let roots = SourceRootRepo::new(catalog.pools().clone());
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();

        ScanService::new(catalog.pools().clone(), thumb_dir)
            .scan_root(root.id, &ScanControl::noop())
            .await
            .unwrap();

        let link = LinkService::new(catalog.pools().clone());
        let count = link.link_raw_jpeg_in_root(root.id).await.unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn sha256_bytes_matches_file_hash() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("hash.jpg");
        std::fs::write(&file, include_bytes!("../../tests/fixtures/minimal.jpg")).unwrap();
        let from_file = file_sha256(&file).unwrap();
        let from_bytes = sha256_bytes(
            &std::fs::read(&file).unwrap(),
        );
        assert_eq!(from_file, from_bytes);
    }

    #[tokio::test]
    async fn recompute_content_hash_updates_stored_hash() {
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        let jpeg = include_bytes!("../../tests/fixtures/minimal.jpg");
        let file_path = photos.join("sample.jpg");
        std::fs::write(&file_path, jpeg).unwrap();

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

        let link = LinkService::new(pools.clone());
        link.compute_hashes_for_root(root.id, &photos)
            .await
            .unwrap();

        let asset = sqlx::query_as::<_, HashRow>(
            "SELECT id, content_hash, rel_path FROM asset WHERE root_id = ? LIMIT 1",
        )
        .bind(root.id)
        .fetch_one(pools.read())
        .await
        .unwrap();
        let before = asset.content_hash.clone().expect("hash");

        std::fs::write(&file_path, b"changed-bytes").unwrap();
        link.recompute_content_hash(asset.id, &file_path)
            .await
            .unwrap();

        let after = sqlx::query_scalar::<_, String>("SELECT content_hash FROM asset WHERE id = ?")
            .bind(asset.id)
            .fetch_one(pools.read())
            .await
            .unwrap();
        assert_ne!(before, after);
    }

    async fn seed_duplicate_assets(
        dir: &tempfile::TempDir,
    ) -> (crate::catalog::pools::CatalogPools, i64, i64, i64) {
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        let jpeg = include_bytes!("../../tests/fixtures/minimal.jpg");
        std::fs::write(photos.join("a.jpg"), jpeg).unwrap();
        std::fs::write(photos.join("b.jpg"), jpeg).unwrap();

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
        let link = LinkService::new(pools.clone());
        link.compute_hashes_for_root(root.id, &photos)
            .await
            .unwrap();
        let ids = sqlx::query_scalar::<_, i64>(
            "SELECT id FROM asset WHERE root_id = ? ORDER BY rel_path",
        )
        .bind(root.id)
        .fetch_all(pools.read())
        .await
        .unwrap();
        (pools, root.id, ids[0], ids[1])
    }

    #[tokio::test]
    async fn find_duplicates_without_root_scope() {
        let dir = tempdir().unwrap();
        let (pools, _root, _a, _b) = seed_duplicate_assets(&dir).await;
        let link = LinkService::new(pools);
        let groups = link.find_duplicates_by_hash(None).await.unwrap();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].len(), 2);
    }

    #[tokio::test]
    async fn rebuild_and_refresh_duplicate_index() {
        let dir = tempdir().unwrap();
        let (pools, root, a, b) = seed_duplicate_assets(&dir).await;
        let link = LinkService::new(pools.clone());
        link.rebuild_duplicate_links().await.unwrap();
        link.refresh_duplicate_flags().await.unwrap();
        link.refresh_duplicate_index().await.unwrap();

        let flagged: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM asset WHERE has_duplicate = 1 AND root_id = ?",
        )
        .bind(root)
        .fetch_one(pools.read())
        .await
        .unwrap();
        assert_eq!(flagged, 2);

        let peers = link.list_duplicates_for_asset(a).await.unwrap();
        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0].id, b);

        let linked = link.link_duplicates_in_root(root).await.unwrap();
        assert_eq!(linked, 1);
        let links = link.list_links(a).await.unwrap();
        assert!(links.is_empty());
    }

    #[tokio::test]
    async fn list_links_returns_raw_jpeg_pair() {
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        let jpeg = include_bytes!("../../tests/fixtures/minimal.jpg");
        std::fs::write(photos.join("DSC010.arw"), jpeg).unwrap();
        std::fs::write(photos.join("DSC010.jpg"), jpeg).unwrap();

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
        let link = LinkService::new(pools.clone());
        link.link_raw_jpeg_in_root(root.id).await.unwrap();

        let raw_id: i64 =
            sqlx::query_scalar("SELECT id FROM asset WHERE root_id = ? AND kind = 'raw' LIMIT 1")
                .bind(root.id)
                .fetch_one(pools.read())
                .await
                .unwrap();
        let links = link.list_links(raw_id).await.unwrap();
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].kind, "raw_jpeg");
    }

    #[test]
    fn stem_name_normalizes_case_and_extension() {
        assert_eq!(stem_name("DSC001.JPG"), "dsc001");
        assert_eq!(stem_name("noext"), "noext");
    }

    #[tokio::test]
    async fn compute_hashes_for_root_returns_zero_when_files_missing() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let pools = catalog.pools().clone();
        let roots = SourceRootRepo::new(pools.clone());
        let root = roots
            .insert_root(dir.path().to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO asset (root_id, rel_path, file_name, ext, kind, size, mtime_ns, sync_state) VALUES (?, 'missing.jpg', 'missing.jpg', 'jpg', 'image', 1, 1, 'new')",
        )
        .bind(root.id)
        .execute(pools.write())
        .await
        .unwrap();
        let link = LinkService::new(pools);
        let hashed = link
            .compute_hashes_for_root(root.id, dir.path())
            .await
            .unwrap();
        assert_eq!(hashed, 0);
    }

    #[tokio::test]
    async fn list_duplicates_for_asset_returns_empty_without_peers() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let pools = catalog.pools().clone();
        let link = LinkService::new(pools);
        let peers = link.list_duplicates_for_asset(999_999).await.unwrap();
        assert!(peers.is_empty());
    }

    #[tokio::test]
    async fn rebuild_duplicate_links_skips_singleton_groups() {
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        std::fs::write(
            photos.join("solo.jpg"),
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let pools = catalog.pools().clone();
        let roots = SourceRootRepo::new(pools.clone());
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        ScanService::new(pools.clone(), dir.path().join("thumbs"))
            .scan_root(root.id, &ScanControl::noop())
            .await
            .unwrap();
        let link = LinkService::new(pools);
        link.compute_hashes_for_root(root.id, &photos)
            .await
            .unwrap();
        link.rebuild_duplicate_links().await.unwrap();
        let linked = link.link_duplicates_in_root(root.id).await.unwrap();
        assert_eq!(linked, 0);
    }

    #[tokio::test]
    async fn link_raw_jpeg_matches_jpeg_extension() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let pools = catalog.pools().clone();
        let roots = SourceRootRepo::new(pools.clone());
        let root = roots
            .insert_root(dir.path().to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO asset (root_id, rel_path, file_name, ext, kind, size, mtime_ns, sync_state) VALUES (?, 'a.arw', 'a.arw', 'arw', 'raw', 1, 1, 'ok')",
        )
        .bind(root.id)
        .execute(pools.write())
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO asset (root_id, rel_path, file_name, ext, kind, size, mtime_ns, sync_state) VALUES (?, 'a.jpeg', 'a.jpeg', 'jpeg', 'image', 1, 1, 'ok')",
        )
        .bind(root.id)
        .execute(pools.write())
        .await
        .unwrap();
        let link = LinkService::new(pools);
        assert_eq!(link.link_raw_jpeg_in_root(root.id).await.unwrap(), 1);
    }

    #[tokio::test]
    async fn find_duplicates_scoped_to_single_root() {
        let dir = tempdir().unwrap();
        let (pools, root_a, a, _b) = seed_duplicate_assets(&dir).await;
        let photos_b = dir.path().join("photos-b");
        std::fs::create_dir_all(&photos_b).unwrap();
        std::fs::write(
            photos_b.join("dup.jpg"),
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();
        let roots = SourceRootRepo::new(pools.clone());
        let root_b = roots
            .insert_root(photos_b.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        ScanService::new(pools.clone(), dir.path().join("thumbs-b"))
            .scan_root(root_b.id, &ScanControl::noop())
            .await
            .unwrap();
        let link = LinkService::new(pools);
        link.compute_hashes_for_root(root_b.id, &photos_b)
            .await
            .unwrap();
        let groups = link.find_duplicates_by_hash(Some(root_a)).await.unwrap();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0][0], a);
    }

    #[tokio::test]
    async fn find_duplicates_by_hash_skips_rows_without_hash() {
        let (catalog, dir) = test_catalog().await;
        let pools = catalog.pools().clone();
        let roots = SourceRootRepo::new(pools.clone());
        let root = roots
            .insert_root(dir.path().to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO asset (root_id, rel_path, file_name, ext, kind, size, mtime_ns, sync_state, content_hash) VALUES (?, 'solo.jpg', 'solo.jpg', 'jpg', 'image', 1, 1, 'ok', NULL)",
        )
        .bind(root.id)
        .execute(pools.write())
        .await
        .unwrap();
        let link = LinkService::new(pools);
        let groups = link.find_duplicates_by_hash(None).await.unwrap();
        assert!(groups.is_empty());
    }

    #[tokio::test]
    async fn link_raw_jpeg_prefers_first_jpeg_by_rel_path() {
        let (catalog, dir) = test_catalog().await;
        let pools = catalog.pools().clone();
        let roots = SourceRootRepo::new(pools.clone());
        let root = roots
            .insert_root(dir.path().to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        for (rel_path, file_name, kind, ext) in [
            ("z/shot.arw", "shot.arw", "raw", "arw"),
            ("b/shot.jpg", "shot.jpg", "image", "jpg"),
            ("a/shot.jpg", "shot.jpg", "image", "jpg"),
        ] {
            sqlx::query(
                "INSERT INTO asset (root_id, rel_path, file_name, ext, kind, size, mtime_ns, sync_state) VALUES (?, ?, ?, ?, ?, 1, 1, 'ok')",
            )
            .bind(root.id)
            .bind(rel_path)
            .bind(file_name)
            .bind(ext)
            .bind(kind)
            .execute(pools.write())
            .await
            .unwrap();
        }
        let link = LinkService::new(pools.clone());
        assert_eq!(link.link_raw_jpeg_in_root(root.id).await.unwrap(), 1);
        let raw_id: i64 =
            sqlx::query_scalar("SELECT id FROM asset WHERE root_id = ? AND kind = 'raw' LIMIT 1")
                .bind(root.id)
                .fetch_one(pools.read())
                .await
                .unwrap();
        let first_jpeg_id: i64 = sqlx::query_scalar(
            "SELECT id FROM asset WHERE root_id = ? AND rel_path = 'a/shot.jpg'",
        )
        .bind(root.id)
        .fetch_one(pools.read())
        .await
        .unwrap();
        let links = link.list_links(raw_id).await.unwrap();
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].id, first_jpeg_id);
    }

    #[tokio::test]
    async fn compute_hashes_for_root_hashes_present_files() {
        let (catalog, dir) = test_catalog().await;
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        std::fs::write(
            photos.join("hash-me.jpg"),
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();
        let pools = catalog.pools().clone();
        let roots = SourceRootRepo::new(pools.clone());
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        ScanService::new(pools.clone(), dir.path().join("thumbs"))
            .scan_root(root.id, &ScanControl::noop())
            .await
            .unwrap();
        sqlx::query("UPDATE asset SET content_hash = NULL WHERE root_id = ?")
            .bind(root.id)
            .execute(pools.write())
            .await
            .unwrap();
        let link = LinkService::new(pools.clone());
        let hashed = link
            .compute_hashes_for_root(root.id, &photos)
            .await
            .unwrap();
        assert_eq!(hashed, 1);
        let stored: Option<String> =
            sqlx::query_scalar("SELECT content_hash FROM asset WHERE root_id = ? LIMIT 1")
                .bind(root.id)
                .fetch_one(pools.read())
                .await
                .unwrap();
        assert!(stored.is_some());
    }

    #[tokio::test]
    async fn refresh_duplicate_index_runs_rebuild_and_refresh() {
        let dir = tempdir().unwrap();
        let (pools, root, a, b) = seed_duplicate_assets(&dir).await;
        let link = LinkService::new(pools.clone());
        link.refresh_duplicate_index().await.unwrap();
        let flagged: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM asset WHERE has_duplicate = 1 AND root_id = ?",
        )
        .bind(root)
        .fetch_one(pools.read())
        .await
        .unwrap();
        assert_eq!(flagged, 2);
        let links: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM asset_link WHERE kind = 'duplicate_hash'")
                .fetch_one(pools.read())
                .await
                .unwrap();
        assert_eq!(links, 1);
        let peers = link.list_duplicates_for_asset(a).await.unwrap();
        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0].id, b);
    }

    #[tokio::test]
    async fn compute_hashes_for_root_rehashes_modified_assets() {
        let (catalog, dir) = test_catalog().await;
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        let file = photos.join("changed.jpg");
        std::fs::write(&file, include_bytes!("../../tests/fixtures/minimal.jpg")).unwrap();
        let pools = catalog.pools().clone();
        let roots = SourceRootRepo::new(pools.clone());
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        ScanService::new(pools.clone(), dir.path().join("thumbs"))
            .scan_root(root.id, &ScanControl::noop())
            .await
            .unwrap();
        let before: String =
            sqlx::query_scalar("SELECT content_hash FROM asset WHERE root_id = ? LIMIT 1")
                .bind(root.id)
                .fetch_one(pools.read())
                .await
                .unwrap();
        std::fs::write(&file, b"changed-bytes").unwrap();
        sqlx::query("UPDATE asset SET sync_state = 'modified' WHERE root_id = ?")
            .bind(root.id)
            .execute(pools.write())
            .await
            .unwrap();
        let link = LinkService::new(pools.clone());
        assert_eq!(
            link.compute_hashes_for_root(root.id, &photos)
                .await
                .unwrap(),
            1
        );
        let after: String =
            sqlx::query_scalar("SELECT content_hash FROM asset WHERE root_id = ? LIMIT 1")
                .bind(root.id)
                .fetch_one(pools.read())
                .await
                .unwrap();
        assert_ne!(before, after);
    }

    #[tokio::test]
    async fn insert_link_is_idempotent_for_duplicate_pairs() {
        let dir = tempdir().unwrap();
        let (pools, root, a, b) = seed_duplicate_assets(&dir).await;
        let link = LinkService::new(pools.clone());
        let first = link.link_duplicates_in_root(root).await.unwrap();
        let second = link.link_duplicates_in_root(root).await.unwrap();
        assert_eq!(first, 1);
        assert_eq!(second, 1);
        let links: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM asset_link WHERE kind = 'duplicate_hash' AND src_id = ? AND dst_id = ?",
        )
        .bind(a)
        .bind(b)
        .fetch_one(pools.read())
        .await
        .unwrap();
        assert_eq!(links, 1);
    }

    #[tokio::test]
    async fn link_raw_jpeg_errors_under_exclusive_lock() {
        let (catalog, dir) = test_catalog().await;
        let pools = catalog.pools().clone();
        let roots = SourceRootRepo::new(pools.clone());
        let root = roots
            .insert_root(dir.path().to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO asset (root_id, rel_path, file_name, ext, kind, size, mtime_ns, sync_state) VALUES (?, 'a.arw', 'a.arw', 'arw', 'raw', 1, 1, 'ok')",
        )
        .bind(root.id)
        .execute(pools.write())
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO asset (root_id, rel_path, file_name, ext, kind, size, mtime_ns, sync_state) VALUES (?, 'a.jpg', 'a.jpg', 'jpg', 'image', 1, 1, 'ok')",
        )
        .bind(root.id)
        .execute(pools.write())
        .await
        .unwrap();
        let mut locker = pools.write().acquire().await.unwrap();
        sqlx::query("BEGIN EXCLUSIVE")
            .execute(&mut *locker)
            .await
            .unwrap();
        let link = LinkService::new(pools);
        assert!(link.link_raw_jpeg_in_root(root.id).await.is_err());
        sqlx::query("ROLLBACK").execute(&mut *locker).await.unwrap();
    }

    #[tokio::test]
    async fn recompute_content_hash_errors_for_missing_file() {
        let (catalog, dir) = test_catalog().await;
        let pools = catalog.pools().clone();
        let roots = SourceRootRepo::new(pools.clone());
        let root = roots
            .insert_root(dir.path().to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        let asset_id: i64 = sqlx::query_scalar(
            "INSERT INTO asset (root_id, rel_path, file_name, ext, kind, size, mtime_ns, sync_state) VALUES (?, 'gone.jpg', 'gone.jpg', 'jpg', 'image', 1, 1, 'ok') RETURNING id",
        )
        .bind(root.id)
        .fetch_one(pools.write())
        .await
        .unwrap();
        let link = LinkService::new(pools);
        assert!(link
            .recompute_content_hash(asset_id, &dir.path().join("gone.jpg"))
            .await
            .is_err());
    }

    #[tokio::test]
    async fn find_duplicates_errors_after_pool_close() {
        let dir = tempdir().unwrap();
        let (pools, root, _a, _b) = seed_duplicate_assets(&dir).await;
        pools.close().await;
        let link = LinkService::new(pools);
        assert!(link.find_duplicates_by_hash(Some(root)).await.is_err());
        assert!(link.find_duplicates_by_hash(None).await.is_err());
    }

    #[tokio::test]
    async fn list_links_errors_after_pool_close() {
        let dir = tempdir().unwrap();
        let (pools, _root, a, _b) = seed_duplicate_assets(&dir).await;
        pools.close().await;
        let link = LinkService::new(pools);
        assert!(link.list_links(a).await.is_err());
    }

    #[tokio::test]
    async fn refresh_duplicate_flags_errors_under_exclusive_lock() {
        let dir = tempdir().unwrap();
        let (pools, _root, _a, _b) = seed_duplicate_assets(&dir).await;
        let mut locker = pools.write().acquire().await.unwrap();
        sqlx::query("BEGIN EXCLUSIVE")
            .execute(&mut *locker)
            .await
            .unwrap();
        let link = LinkService::new(pools);
        assert!(link.refresh_duplicate_flags().await.is_err());
        sqlx::query("ROLLBACK").execute(&mut *locker).await.unwrap();
    }

    #[tokio::test]
    async fn rebuild_duplicate_links_errors_under_exclusive_lock() {
        let dir = tempdir().unwrap();
        let (pools, _root, _a, _b) = seed_duplicate_assets(&dir).await;
        let mut locker = pools.write().acquire().await.unwrap();
        sqlx::query("BEGIN EXCLUSIVE")
            .execute(&mut *locker)
            .await
            .unwrap();
        let link = LinkService::new(pools);
        assert!(link.rebuild_duplicate_links().await.is_err());
        sqlx::query("ROLLBACK").execute(&mut *locker).await.unwrap();
    }

    #[tokio::test]
    async fn list_duplicates_for_asset_errors_after_pool_close() {
        let dir = tempdir().unwrap();
        let (pools, _root, a, _b) = seed_duplicate_assets(&dir).await;
        pools.close().await;
        let link = LinkService::new(pools);
        assert!(link.list_duplicates_for_asset(a).await.is_err());
    }

    #[tokio::test]
    async fn compute_hashes_for_root_errors_after_pool_close() {
        let dir = tempdir().unwrap();
        let (pools, root, _a, _b) = seed_duplicate_assets(&dir).await;
        pools.close().await;
        let link = LinkService::new(pools);
        assert!(link
            .compute_hashes_for_root(root, &dir.path().join("photos"))
            .await
            .is_err());
    }
}
