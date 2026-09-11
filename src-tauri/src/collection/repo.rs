use crate::catalog::models::{Album, SmartCollection, SmartCollectionRow};
use crate::error::{AppError, Result};
use crate::query::AssetFilter;
use crate::sort::SortSpec;
use sqlx::SqlitePool;

const ALBUM_ASSET_COUNT_SQL: &str = r#"
COALESCE((
  SELECT COUNT(*)
  FROM album_item ai
  JOIN asset ast ON ast.id = ai.asset_id AND ast.deleted_at IS NULL
  WHERE ai.album_id = a.id
), 0) AS asset_count
"#;

pub struct CollectionRepo {
    pool: SqlitePool,
}

impl CollectionRepo {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    fn row_to_smart_collection(row: SmartCollectionRow) -> Result<SmartCollection> {
        let filter = serde_json::from_str(&row.filter_json)
            .map_err(|e| AppError::InvalidInput(e.to_string()))?;
        validate_smart_filter(&filter)?;
        Ok(SmartCollection {
            id: row.id,
            name: row.name,
            filter,
            asset_count: 0,
        })
    }

    pub async fn list_smart_collections(&self) -> Result<Vec<SmartCollection>> {
        let rows = sqlx::query_as::<_, SmartCollectionRow>(
            "SELECT id, name, filter_json FROM smart_collection ORDER BY name",
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(Self::row_to_smart_collection)
            .collect()
    }

    pub async fn get_smart_collection(&self, id: i64) -> Result<SmartCollection> {
        let row = sqlx::query_as::<_, SmartCollectionRow>(
            "SELECT id, name, filter_json FROM smart_collection WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("smart collection {}", id)))?;
        Self::row_to_smart_collection(row)
    }

    pub async fn save_smart_collection(
        &self,
        name: &str,
        filter: &AssetFilter,
    ) -> Result<SmartCollection> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(AppError::InvalidInput("collection name is required".into()));
        }
        validate_smart_filter(filter)?;
        let filter_json = serde_json::to_string(filter)?;

        let existing =
            sqlx::query_scalar::<_, i64>("SELECT id FROM smart_collection WHERE name = ?")
                .bind(trimmed)
                .fetch_optional(&self.pool)
                .await?;

        if let Some(id) = existing {
            sqlx::query("UPDATE smart_collection SET filter_json = ? WHERE id = ?")
                .bind(&filter_json)
                .bind(id)
                .execute(&self.pool)
                .await?;
        } else {
            sqlx::query("INSERT INTO smart_collection (name, filter_json) VALUES (?, ?)")
                .bind(trimmed)
                .bind(&filter_json)
                .execute(&self.pool)
                .await?;
        }

        let row = sqlx::query_as::<_, SmartCollectionRow>(
            "SELECT id, name, filter_json FROM smart_collection WHERE name = ?",
        )
        .bind(trimmed)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Catalog(e.to_string()))?;
        Self::row_to_smart_collection(row)
    }

    pub async fn delete_smart_collection(&self, id: i64) -> Result<()> {
        let r = sqlx::query("DELETE FROM smart_collection WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        if r.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("smart collection {}", id)));
        }
        Ok(())
    }

    pub async fn list_albums(&self) -> Result<Vec<Album>> {
        let sql = format!(
            "SELECT a.id, a.name, a.sort_mode, a.emoji, {ALBUM_ASSET_COUNT_SQL} FROM album a ORDER BY a.name"
        );
        Ok(sqlx::query_as::<_, Album>(&sql)
            .fetch_all(&self.pool)
            .await?)
    }

    async fn fetch_album(&self, id: i64) -> Result<Album> {
        let sql = format!(
            "SELECT a.id, a.name, a.sort_mode, a.emoji, {ALBUM_ASSET_COUNT_SQL} FROM album a WHERE a.id = ?"
        );
        sqlx::query_as::<_, Album>(&sql)
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::Catalog(e.to_string()))
    }

    pub async fn create_album(
        &self,
        name: &str,
        sort_mode: &str,
        emoji: Option<&str>,
    ) -> Result<Album> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(AppError::InvalidInput("album name is required".into()));
        }
        SortSpec::parse(sort_mode)?;

        let id = sqlx::query_scalar::<_, i64>(
            "INSERT INTO album (name, sort_mode, emoji) VALUES (?, ?, ?) RETURNING id",
        )
        .bind(trimmed)
        .bind(sort_mode)
        .bind(emoji)
        .fetch_one(&self.pool)
        .await?;

        self.fetch_album(id).await
    }

    pub async fn delete_album(&self, id: i64) -> Result<()> {
        let r = sqlx::query("DELETE FROM album WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        if r.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("album {}", id)));
        }
        Ok(())
    }

    pub async fn set_album_items(&self, album_id: i64, asset_ids: &[i64]) -> Result<()> {
        sqlx::query("DELETE FROM album_item WHERE album_id = ?")
            .bind(album_id)
            .execute(&self.pool)
            .await?;
        if asset_ids.is_empty() {
            return Ok(());
        }
        let mut builder =
            sqlx::QueryBuilder::new("INSERT INTO album_item (album_id, asset_id, position) ");
        builder.push_values(asset_ids.iter().enumerate(), |mut b, (pos, asset_id)| {
            b.push_bind(album_id)
                .push_bind(*asset_id)
                .push_bind(pos as i64);
        });
        builder.build().execute(&self.pool).await?;
        Ok(())
    }

    pub async fn add_album_items(&self, album_id: i64, asset_ids: &[i64]) -> Result<u64> {
        if asset_ids.is_empty() {
            return Ok(0);
        }
        let existing = self.album_asset_ids(album_id).await?;
        let mut seen: std::collections::HashSet<i64> = existing.iter().copied().collect();
        let mut merged = existing;
        let mut added = 0u64;
        for asset_id in asset_ids {
            if seen.insert(*asset_id) {
                merged.push(*asset_id);
                added += 1;
            }
        }
        if added > 0 {
            self.set_album_items(album_id, &merged).await?;
        }
        Ok(added)
    }

    pub async fn remove_album_items(&self, album_id: i64, asset_ids: &[i64]) -> Result<u64> {
        if asset_ids.is_empty() {
            return Ok(0);
        }
        let existing = self.album_asset_ids(album_id).await?;
        let remove_set: std::collections::HashSet<i64> = asset_ids.iter().copied().collect();
        let mut removed = 0u64;
        let filtered: Vec<i64> = existing
            .into_iter()
            .filter(|asset_id| {
                if remove_set.contains(asset_id) {
                    removed += 1;
                    false
                } else {
                    true
                }
            })
            .collect();
        if removed > 0 {
            self.set_album_items(album_id, &filtered).await?;
        }
        Ok(removed)
    }

    pub async fn album_asset_ids(&self, album_id: i64) -> Result<Vec<i64>> {
        let sort_mode = sqlx::query_scalar::<_, String>("SELECT sort_mode FROM album WHERE id = ?")
            .bind(album_id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("album {}", album_id)))?;

        let sort = SortSpec::parse(&sort_mode)?;
        let order = sort.album_order_sql();

        let sql = format!(
            r#"
            SELECT ai.asset_id FROM album_item ai
            JOIN asset a ON a.id = ai.asset_id
            LEFT JOIN asset_meta m ON m.asset_id = a.id
            WHERE ai.album_id = ? AND a.deleted_at IS NULL
            ORDER BY {}
            "#,
            order
        );

        Ok(sqlx::query_scalar::<_, i64>(&sql)
            .bind(album_id)
            .fetch_all(&self.pool)
            .await?)
    }
    pub async fn list_album_ids_for_asset(&self, asset_id: i64) -> Result<Vec<i64>> {
        Ok(sqlx::query_scalar::<_, i64>(
            "SELECT album_id FROM album_item WHERE asset_id = ? ORDER BY album_id",
        )
        .bind(asset_id)
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn update_album(&self, id: i64, name: &str, emoji: Option<&str>) -> Result<Album> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(AppError::InvalidInput("album name is required".into()));
        }
        let result = sqlx::query("UPDATE album SET name = ?, emoji = ? WHERE id = ?")
            .bind(trimmed)
            .bind(emoji)
            .bind(id)
            .execute(&self.pool)
            .await?;
        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("album {}", id)));
        }
        self.fetch_album(id).await
    }
}

fn validate_smart_filter(filter: &AssetFilter) -> Result<()> {
    if filter.asset_ids.is_some() {
        return Err(AppError::InvalidInput(
            "smart collections cannot filter by asset ids".into(),
        ));
    }
    if filter.deleted_only == Some(true) {
        return Err(AppError::InvalidInput(
            "smart collections cannot filter deleted assets".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::Catalog;
    use tempfile::tempdir;

    #[tokio::test]
    async fn smart_collection_roundtrip_and_validation() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let repo = CollectionRepo::new(catalog.pool().clone());
        let filter = AssetFilter {
            rating_min: Some(3),
            ..Default::default()
        };
        let saved = repo.save_smart_collection("rated", &filter).await.unwrap();
        let listed = repo.list_smart_collections().await.unwrap();
        assert!(listed.iter().any(|row| row.id == saved.id));
        let fetched = repo.get_smart_collection(saved.id).await.unwrap();
        assert_eq!(fetched.name, "rated");
        repo.delete_smart_collection(saved.id).await.unwrap();
        assert!(repo.get_smart_collection(saved.id).await.is_err());
    }

    #[tokio::test]
    async fn smart_collection_rejects_invalid_filters() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let repo = CollectionRepo::new(catalog.pool().clone());
        let err = repo
            .save_smart_collection(
                "bad",
                &AssetFilter {
                    asset_ids: Some(vec![1]),
                    ..Default::default()
                },
            )
            .await
            .unwrap_err();
        assert!(err.to_string().contains("asset ids"));
    }

    #[tokio::test]
    async fn album_crud_and_items() {
        use crate::catalog::repo::{AssetRepo, SourceRootRepo, UpsertAssetInput};

        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let repo = CollectionRepo::new(catalog.pool().clone());
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root(dir.path().to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        let assets = AssetRepo::new(catalog.pool().clone());
        let one = assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "one.jpg".into(),
                file_name: "one.jpg".into(),
                ext: "jpg".into(),
                kind: "image".into(),
                size: 1,
                mtime_ns: 1,
                sync_state: "ok",
            })
            .await
            .unwrap();
        let two = assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "two.jpg".into(),
                file_name: "two.jpg".into(),
                ext: "jpg".into(),
                kind: "image".into(),
                size: 1,
                mtime_ns: 1,
                sync_state: "ok",
            })
            .await
            .unwrap();
        let three = assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "three.jpg".into(),
                file_name: "three.jpg".into(),
                ext: "jpg".into(),
                kind: "image".into(),
                size: 1,
                mtime_ns: 1,
                sync_state: "ok",
            })
            .await
            .unwrap();
        let album = repo
            .create_album("Trip", "date:desc", Some("📷"))
            .await
            .unwrap();
        assert_eq!(album.name, "Trip");
        repo.set_album_items(album.id, &[one.id, two.id])
            .await
            .unwrap();
        let ids = repo.album_asset_ids(album.id).await.unwrap();
        assert_eq!(ids, vec![one.id, two.id]);
        let added = repo
            .add_album_items(album.id, &[two.id, three.id])
            .await
            .unwrap();
        assert_eq!(added, 1);
        let removed = repo.remove_album_items(album.id, &[one.id]).await.unwrap();
        assert_eq!(removed, 1);
        let updated = repo
            .update_album(album.id, "Trip 2", Some("🎞"))
            .await
            .unwrap();
        assert_eq!(updated.name, "Trip 2");
        assert!(repo.update_album(999, "x", None).await.is_err());
        assert!(repo.create_album("", "date:desc", None).await.is_err());
        assert_eq!(repo.add_album_items(album.id, &[]).await.unwrap(), 0);
        assert_eq!(repo.remove_album_items(album.id, &[]).await.unwrap(), 0);
        repo.delete_album(album.id).await.unwrap();
        let albums = repo.list_albums().await.unwrap();
        assert!(albums.iter().all(|row| row.id != album.id));
        assert!(repo.delete_album(999).await.is_err());
    }

    #[tokio::test]
    async fn smart_collection_updates_existing_and_validates_name() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let repo = CollectionRepo::new(catalog.pool().clone());
        let first = repo
            .save_smart_collection(
                "rated",
                &AssetFilter {
                    rating_min: Some(3),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        let second = repo
            .save_smart_collection(
                "rated",
                &AssetFilter {
                    rating_min: Some(5),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(first.id, second.id);
        assert_eq!(second.filter.rating_min, Some(5));
        assert!(repo
            .save_smart_collection("", &AssetFilter::default())
            .await
            .is_err());
        assert!(repo.delete_smart_collection(999).await.is_err());
    }

    #[tokio::test]
    async fn smart_collection_rejects_deleted_only_filter() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let repo = CollectionRepo::new(catalog.pool().clone());
        let err = repo
            .save_smart_collection(
                "trash",
                &AssetFilter {
                    deleted_only: Some(true),
                    ..Default::default()
                },
            )
            .await
            .unwrap_err();
        assert!(err.to_string().contains("deleted"));
    }

    #[tokio::test]
    async fn remove_album_items_updates_positions() {
        use crate::catalog::repo::{AssetRepo, SourceRootRepo, UpsertAssetInput};

        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let repo = CollectionRepo::new(catalog.pool().clone());
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root(dir.path().to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        let assets = AssetRepo::new(catalog.pool().clone());
        let one = assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "one.jpg".into(),
                file_name: "one.jpg".into(),
                ext: "jpg".into(),
                kind: "image".into(),
                size: 1,
                mtime_ns: 1,
                sync_state: "ok",
            })
            .await
            .unwrap();
        let two = assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "two.jpg".into(),
                file_name: "two.jpg".into(),
                ext: "jpg".into(),
                kind: "image".into(),
                size: 1,
                mtime_ns: 1,
                sync_state: "ok",
            })
            .await
            .unwrap();
        let album = repo.create_album("Trip", "date:desc", None).await.unwrap();
        repo.set_album_items(album.id, &[one.id, two.id])
            .await
            .unwrap();
        let removed = repo.remove_album_items(album.id, &[one.id]).await.unwrap();
        assert_eq!(removed, 1);
        assert_eq!(repo.album_asset_ids(album.id).await.unwrap(), vec![two.id]);
    }

    #[tokio::test]
    async fn create_album_rejects_invalid_sort_mode() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let repo = CollectionRepo::new(catalog.pool().clone());
        let err = repo
            .create_album("Trip", "bad:sort", None)
            .await
            .unwrap_err();
        assert!(err.to_string().contains("sort"));
    }

    #[tokio::test]
    async fn smart_collection_rejects_corrupt_filter_json() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let repo = CollectionRepo::new(catalog.pool().clone());
        sqlx::query("INSERT INTO smart_collection (name, filter_json) VALUES (?, ?)")
            .bind("bad")
            .bind("{not-json")
            .execute(catalog.pool())
            .await
            .unwrap();
        let err = repo.list_smart_collections().await.unwrap_err();
        assert!(err.to_string().contains("InvalidInput") || !err.to_string().is_empty());
    }

    #[tokio::test]
    async fn album_asset_ids_requires_existing_album() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let repo = CollectionRepo::new(catalog.pool().clone());
        let err = repo.album_asset_ids(999).await.unwrap_err();
        assert!(err.to_string().contains("album"));
    }

    #[tokio::test]
    async fn remove_album_items_returns_zero_when_no_matches() {
        use crate::catalog::repo::{AssetRepo, SourceRootRepo, UpsertAssetInput};

        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let repo = CollectionRepo::new(catalog.pool().clone());
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root(dir.path().to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        let assets = AssetRepo::new(catalog.pool().clone());
        let asset = assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "one.jpg".into(),
                file_name: "one.jpg".into(),
                ext: "jpg".into(),
                kind: "image".into(),
                size: 1,
                mtime_ns: 1,
                sync_state: "ok",
            })
            .await
            .unwrap();
        let album = repo.create_album("Trip", "name:asc", None).await.unwrap();
        repo.set_album_items(album.id, &[asset.id]).await.unwrap();
        assert_eq!(
            repo.remove_album_items(album.id, &[asset.id + 999])
                .await
                .unwrap(),
            0
        );
        assert_eq!(
            repo.add_album_items(album.id, &[asset.id]).await.unwrap(),
            0
        );
    }

    #[tokio::test]
    async fn update_album_rejects_empty_name() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let repo = CollectionRepo::new(catalog.pool().clone());
        let album = repo.create_album("Trip", "date:desc", None).await.unwrap();
        assert!(repo.update_album(album.id, "  ", None).await.is_err());
    }

    #[tokio::test]
    async fn get_smart_collection_not_found_before_insert() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let repo = CollectionRepo::new(catalog.pool().clone());
        assert!(repo.get_smart_collection(999).await.is_err());
    }

    #[tokio::test]
    async fn set_album_items_clear_removes_all_members() {
        use crate::catalog::repo::{AssetRepo, SourceRootRepo, UpsertAssetInput};

        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let repo = CollectionRepo::new(catalog.pool().clone());
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root(dir.path().to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        let assets = AssetRepo::new(catalog.pool().clone());
        let asset = assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "one.jpg".into(),
                file_name: "one.jpg".into(),
                ext: "jpg".into(),
                kind: "image".into(),
                size: 1,
                mtime_ns: 1,
                sync_state: "ok",
            })
            .await
            .unwrap();
        let album = repo.create_album("Trip", "date:desc", None).await.unwrap();
        repo.set_album_items(album.id, &[asset.id]).await.unwrap();
        repo.set_album_items(album.id, &[]).await.unwrap();
        assert!(repo.album_asset_ids(album.id).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn list_album_ids_for_asset_returns_memberships() {
        use crate::catalog::repo::{AssetRepo, SourceRootRepo, UpsertAssetInput};

        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let repo = CollectionRepo::new(catalog.pool().clone());
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root(dir.path().to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        let assets = AssetRepo::new(catalog.pool().clone());
        let asset = assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "one.jpg".into(),
                file_name: "one.jpg".into(),
                ext: "jpg".into(),
                kind: "image".into(),
                size: 1,
                mtime_ns: 1,
                sync_state: "ok",
            })
            .await
            .unwrap();
        let first = repo.create_album("First", "name:asc", None).await.unwrap();
        let second = repo
            .create_album("Second", "path:desc", None)
            .await
            .unwrap();
        repo.set_album_items(first.id, &[asset.id]).await.unwrap();
        repo.set_album_items(second.id, &[asset.id]).await.unwrap();
        let album_ids = repo.list_album_ids_for_asset(asset.id).await.unwrap();
        assert_eq!(album_ids, vec![first.id, second.id]);
        assert_eq!(
            repo.album_asset_ids(first.id).await.unwrap(),
            vec![asset.id]
        );
        assert_eq!(
            repo.album_asset_ids(second.id).await.unwrap(),
            vec![asset.id]
        );
    }

    #[tokio::test]
    async fn get_smart_collection_rejects_corrupt_filter_json() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let repo = CollectionRepo::new(catalog.pool().clone());
        let id = sqlx::query_scalar::<_, i64>(
            "INSERT INTO smart_collection (name, filter_json) VALUES (?, ?) RETURNING id",
        )
        .bind("broken")
        .bind("{not-json")
        .fetch_one(catalog.pool())
        .await
        .unwrap();
        let err = repo.get_smart_collection(id).await.unwrap_err();
        assert!(!err.to_string().is_empty());
    }

    #[tokio::test]
    async fn smart_collection_list_rejects_invalid_filter_json() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let repo = CollectionRepo::new(catalog.pool().clone());
        sqlx::query("INSERT INTO smart_collection (name, filter_json) VALUES (?, ?)")
            .bind("asset-ids")
            .bind(r#"{"asset_ids":[1]}"#)
            .execute(catalog.pool())
            .await
            .unwrap();
        let err = repo.list_smart_collections().await.unwrap_err();
        assert!(!err.to_string().is_empty());
    }

    #[tokio::test]
    async fn collection_repo_errors_after_pool_close() {
        use crate::catalog::repo::{AssetRepo, SourceRootRepo, UpsertAssetInput};

        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let pool = catalog.pool().clone();
        let repo = CollectionRepo::new(pool.clone());
        let roots = SourceRootRepo::new(pool.clone());
        let root = roots
            .insert_root(dir.path().to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        let assets = AssetRepo::new(pool.clone());
        let asset = assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "one.jpg".into(),
                file_name: "one.jpg".into(),
                ext: "jpg".into(),
                kind: "image".into(),
                size: 1,
                mtime_ns: 1,
                sync_state: "ok",
            })
            .await
            .unwrap();
        let filter = AssetFilter {
            rating_min: Some(3),
            ..Default::default()
        };
        let saved = repo.save_smart_collection("rated", &filter).await.unwrap();
        let album = repo.create_album("Trip", "date:desc", None).await.unwrap();
        repo.set_album_items(album.id, &[asset.id]).await.unwrap();
        pool.close().await;
        assert!(repo.list_smart_collections().await.is_err());
        assert!(repo.get_smart_collection(saved.id).await.is_err());
        assert!(repo.save_smart_collection("rated", &filter).await.is_err());
        assert!(repo.delete_smart_collection(saved.id).await.is_err());
        assert!(repo.list_albums().await.is_err());
        assert!(repo.create_album("Other", "name:asc", None).await.is_err());
        assert!(repo.delete_album(album.id).await.is_err());
        assert!(repo.set_album_items(album.id, &[asset.id]).await.is_err());
        assert!(repo.add_album_items(album.id, &[asset.id]).await.is_err());
        assert!(repo
            .remove_album_items(album.id, &[asset.id])
            .await
            .is_err());
        assert!(repo.album_asset_ids(album.id).await.is_err());
        assert!(repo.list_album_ids_for_asset(asset.id).await.is_err());
        assert!(repo.update_album(album.id, "Trip", None).await.is_err());
    }

    #[tokio::test]
    async fn set_album_items_rejects_unknown_asset_ids() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let repo = CollectionRepo::new(catalog.pool().clone());
        let album = repo.create_album("Trip", "date:desc", None).await.unwrap();
        let err = repo
            .set_album_items(album.id, &[999_999])
            .await
            .unwrap_err();
        assert!(!err.to_string().is_empty());
    }

    #[tokio::test]
    async fn add_album_items_propagates_set_errors() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let repo = CollectionRepo::new(catalog.pool().clone());
        let album = repo.create_album("Trip", "date:desc", None).await.unwrap();
        let err = repo
            .add_album_items(album.id, &[999_999])
            .await
            .unwrap_err();
        assert!(!err.to_string().is_empty());
    }

    #[tokio::test]
    async fn remove_album_items_propagates_set_errors_under_lock() {
        use crate::catalog::repo::{AssetRepo, SourceRootRepo, UpsertAssetInput};

        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let pool = catalog.pool().clone();
        let repo = CollectionRepo::new(pool.clone());
        let roots = SourceRootRepo::new(pool.clone());
        let root = roots
            .insert_root(dir.path().to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        let assets = AssetRepo::new(pool.clone());
        let first = assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "one.jpg".into(),
                file_name: "one.jpg".into(),
                ext: "jpg".into(),
                kind: "image".into(),
                size: 1,
                mtime_ns: 1,
                sync_state: "ok",
            })
            .await
            .unwrap();
        let second = assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "two.jpg".into(),
                file_name: "two.jpg".into(),
                ext: "jpg".into(),
                kind: "image".into(),
                size: 1,
                mtime_ns: 1,
                sync_state: "ok",
            })
            .await
            .unwrap();
        let album = repo.create_album("Trip", "date:desc", None).await.unwrap();
        repo.set_album_items(album.id, &[first.id, second.id])
            .await
            .unwrap();
        let mut locker = pool.acquire().await.unwrap();
        sqlx::query("BEGIN EXCLUSIVE")
            .execute(&mut *locker)
            .await
            .unwrap();
        let err = repo
            .remove_album_items(album.id, &[first.id])
            .await
            .unwrap_err();
        assert!(!err.to_string().is_empty());
        sqlx::query("ROLLBACK").execute(&mut *locker).await.unwrap();
    }

    #[tokio::test]
    async fn update_album_errors_when_album_table_is_dropped() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let repo = CollectionRepo::new(catalog.pool().clone());
        let album = repo.create_album("Trip", "date:desc", None).await.unwrap();
        sqlx::query("DROP TABLE album")
            .execute(catalog.pool())
            .await
            .unwrap();
        let err = repo
            .update_album(album.id, "Trip 2", None)
            .await
            .unwrap_err();
        assert!(!err.to_string().is_empty());
    }

    #[tokio::test]
    async fn update_album_fetch_errors_when_asset_table_is_dropped() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let repo = CollectionRepo::new(catalog.pool().clone());
        let album = repo.create_album("Trip", "date:desc", None).await.unwrap();
        sqlx::query("DROP TABLE asset")
            .execute(catalog.pool())
            .await
            .unwrap();
        let err = repo
            .update_album(album.id, "Trip 2", None)
            .await
            .unwrap_err();
        assert!(!err.to_string().is_empty());
    }

    #[tokio::test]
    async fn save_smart_collection_errors_under_exclusive_lock() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let pool = catalog.pool().clone();
        let repo = CollectionRepo::new(pool.clone());
        let filter = AssetFilter {
            rating_min: Some(3),
            ..Default::default()
        };
        repo.save_smart_collection("rated", &filter).await.unwrap();
        let mut locker = pool.acquire().await.unwrap();
        sqlx::query("BEGIN EXCLUSIVE")
            .execute(&mut *locker)
            .await
            .unwrap();
        let err = repo
            .save_smart_collection(
                "rated",
                &AssetFilter {
                    rating_min: Some(5),
                    ..Default::default()
                },
            )
            .await
            .unwrap_err();
        assert!(!err.to_string().is_empty());
        sqlx::query("ROLLBACK").execute(&mut *locker).await.unwrap();
        sqlx::query("BEGIN EXCLUSIVE")
            .execute(&mut *locker)
            .await
            .unwrap();
        let err = repo
            .save_smart_collection("fresh", &AssetFilter::default())
            .await
            .unwrap_err();
        assert!(!err.to_string().is_empty());
        sqlx::query("ROLLBACK").execute(&mut *locker).await.unwrap();
    }

    #[tokio::test]
    async fn delete_album_removes_only_target_album() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let repo = CollectionRepo::new(catalog.pool().clone());
        let keep = repo.create_album("Keep", "date:desc", None).await.unwrap();
        let remove = repo
            .create_album("Remove", "date:desc", None)
            .await
            .unwrap();
        repo.delete_album(remove.id).await.unwrap();
        let albums = repo.list_albums().await.unwrap();
        assert!(albums.iter().all(|row| row.id != remove.id));
        assert!(albums.iter().any(|row| row.id == keep.id));
    }

    #[tokio::test]
    async fn save_smart_collection_rejects_asset_id_filter() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let repo = CollectionRepo::new(catalog.pool().clone());
        let err = repo
            .save_smart_collection(
                "blocked",
                &AssetFilter {
                    asset_ids: Some(vec![1]),
                    ..Default::default()
                },
            )
            .await
            .unwrap_err();
        assert!(err.to_string().contains("asset ids"));
    }

    #[tokio::test]
    async fn save_smart_collection_fetch_errors_under_exclusive_lock() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let pool = catalog.pool().clone();
        let repo = CollectionRepo::new(pool.clone());
        let filter = AssetFilter {
            rating_min: Some(3),
            ..Default::default()
        };
        repo.save_smart_collection("rated", &filter).await.unwrap();
        let mut locker = pool.acquire().await.unwrap();
        sqlx::query("BEGIN EXCLUSIVE")
            .execute(&mut *locker)
            .await
            .unwrap();
        let err = repo
            .save_smart_collection(
                "rated",
                &AssetFilter {
                    rating_min: Some(5),
                    ..Default::default()
                },
            )
            .await
            .unwrap_err();
        assert!(!err.to_string().is_empty());
        sqlx::query("ROLLBACK").execute(&mut *locker).await.unwrap();
    }

    #[tokio::test]
    async fn list_smart_collections_errors_on_corrupt_filter_json() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        sqlx::query("INSERT INTO smart_collection (name, filter_json) VALUES (?, ?)")
            .bind("broken")
            .bind("{not-json")
            .execute(catalog.pool())
            .await
            .unwrap();
        let repo = CollectionRepo::new(catalog.pool().clone());
        assert!(repo.list_smart_collections().await.is_err());
    }

    #[tokio::test]
    async fn album_asset_ids_errors_when_items_table_missing() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let repo = CollectionRepo::new(catalog.pool().clone());
        let album = repo.create_album("Trip", "date:desc", None).await.unwrap();
        sqlx::query("DROP TABLE album_item")
            .execute(catalog.pool())
            .await
            .unwrap();
        let err = repo.album_asset_ids(album.id).await.unwrap_err();
        assert!(!err.to_string().is_empty());
    }

    #[tokio::test]
    async fn album_asset_ids_rejects_invalid_sort_mode() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let repo = CollectionRepo::new(catalog.pool().clone());
        let album = repo.create_album("Trip", "date:desc", None).await.unwrap();
        sqlx::query("UPDATE album SET sort_mode = ? WHERE id = ?")
            .bind("bad:sort")
            .bind(album.id)
            .execute(catalog.pool())
            .await
            .unwrap();
        let err = repo.album_asset_ids(album.id).await.unwrap_err();
        assert!(err.to_string().contains("sort"));
    }

    #[tokio::test]
    async fn album_asset_ids_supports_rating_sort_mode() {
        use crate::catalog::models::AssetMeta;
        use crate::catalog::repo::{AssetMetaRepo, AssetRepo, SourceRootRepo, UpsertAssetInput};

        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let repo = CollectionRepo::new(catalog.pool().clone());
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root(dir.path().to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        let assets = AssetRepo::new(catalog.pool().clone());
        let meta_repo = AssetMetaRepo::new(catalog.pool().clone());
        let low = assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "low.jpg".into(),
                file_name: "low.jpg".into(),
                ext: "jpg".into(),
                kind: "image".into(),
                size: 1,
                mtime_ns: 1,
                sync_state: "ok",
            })
            .await
            .unwrap();
        let high = assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "high.jpg".into(),
                file_name: "high.jpg".into(),
                ext: "jpg".into(),
                kind: "image".into(),
                size: 1,
                mtime_ns: 1,
                sync_state: "ok",
            })
            .await
            .unwrap();
        meta_repo
            .upsert(&AssetMeta {
                asset_id: low.id,
                capture_at: None,
                camera: None,
                lens: None,
                rating: Some(2),
                latitude: None,
                longitude: None,
                keywords_json: None,
            })
            .await
            .unwrap();
        meta_repo
            .upsert(&AssetMeta {
                asset_id: high.id,
                capture_at: None,
                camera: None,
                lens: None,
                rating: Some(5),
                latitude: None,
                longitude: None,
                keywords_json: None,
            })
            .await
            .unwrap();
        let album = repo
            .create_album("Rated", "rating:desc", None)
            .await
            .unwrap();
        repo.set_album_items(album.id, &[low.id, high.id])
            .await
            .unwrap();
        assert_eq!(
            repo.album_asset_ids(album.id).await.unwrap(),
            vec![high.id, low.id]
        );
    }
}
