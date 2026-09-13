use crate::catalog::pools::CatalogPools;
use crate::error::{AppError, Result};
use sqlx::SqlitePool;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct TagRow {
    pub id: i64,
    pub name: String,
    pub parent_id: Option<i64>,
    pub color: Option<String>,
    pub asset_count: i64,
}

const TAG_ASSET_COUNT_SQL: &str = r#"
COALESCE((
  SELECT COUNT(DISTINCT at.asset_id)
  FROM asset_tag at
  JOIN asset a ON a.id = at.asset_id AND a.deleted_at IS NULL
  WHERE at.tag_id IN (
    WITH RECURSIVE descendants AS (
      SELECT id FROM tag WHERE id = t.id
      UNION ALL
      SELECT child.id FROM tag child INNER JOIN descendants d ON child.parent_id = d.id
    )
    SELECT id FROM descendants
  )
), 0) AS asset_count
"#;

pub struct TagRepo {
    pools: CatalogPools,
}

impl TagRepo {
    pub fn new(pools: CatalogPools) -> Self {
        Self { pools }
    }

    fn read(&self) -> &SqlitePool {
        self.pools.read()
    }

    fn write(&self) -> &SqlitePool {
        self.pools.write()
    }

    pub async fn list_ids_for_asset(&self, asset_id: i64) -> Result<Vec<i64>> {
        Ok(sqlx::query_scalar::<_, i64>(
            r#"
                SELECT t.id FROM tag t
                JOIN asset_tag at ON at.tag_id = t.id
                WHERE at.asset_id = ?
                ORDER BY t.name
                "#,
        )
        .bind(asset_id)
        .fetch_all(self.read())
        .await?)
    }

    pub async fn list_names_for_asset(&self, asset_id: i64) -> Result<Vec<String>> {
        Ok(sqlx::query_scalar::<_, String>(
            r#"
                SELECT t.name FROM tag t
                JOIN asset_tag at ON at.tag_id = t.id
                WHERE at.asset_id = ?
                ORDER BY t.name
                "#,
        )
        .bind(asset_id)
        .fetch_all(self.read())
        .await?)
    }

    pub async fn list_all_names(&self) -> Result<Vec<String>> {
        Ok(
            sqlx::query_scalar::<_, String>("SELECT name FROM tag ORDER BY name")
                .fetch_all(self.read())
                .await?,
        )
    }

    pub async fn list_asset_ids_for_tag(&self, tag_id: i64) -> Result<Vec<i64>> {
        Ok(
            sqlx::query_scalar::<_, i64>("SELECT asset_id FROM asset_tag WHERE tag_id = ?")
                .bind(tag_id)
                .fetch_all(self.read())
                .await?,
        )
    }

    pub async fn list_tag_rows(&self) -> Result<Vec<TagRow>> {
        let sql = format!(
            r#"
            SELECT
              t.id,
              t.name,
              t.parent_id,
              t.color,
              {asset_count}
            FROM tag t
            ORDER BY t.name
            "#,
            asset_count = TAG_ASSET_COUNT_SQL,
        );
        Ok(sqlx::query_as::<_, TagRow>(&sql)
            .fetch_all(self.read())
            .await?)
    }

    pub async fn descendant_tag_ids(&self, tag_id: i64) -> Result<Vec<i64>> {
        Ok(sqlx::query_scalar::<_, i64>(
            r#"
                WITH RECURSIVE descendants AS (
                  SELECT id FROM tag WHERE id = ?
                  UNION ALL
                  SELECT t.id FROM tag t INNER JOIN descendants d ON t.parent_id = d.id
                )
                SELECT id FROM descendants
                "#,
        )
        .bind(tag_id)
        .fetch_all(self.read())
        .await?)
    }

    pub async fn expand_tag_ids_with_descendants(&self, tag_ids: &[i64]) -> Result<Vec<i64>> {
        if tag_ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut expanded = std::collections::HashSet::new();
        for tag_id in tag_ids {
            for id in self.descendant_tag_ids(*tag_id).await? {
                expanded.insert(id);
            }
        }
        let mut ids = expanded.into_iter().collect::<Vec<_>>();
        ids.sort_unstable();
        Ok(ids)
    }

    pub async fn append_tag_id_to_assets(&self, asset_ids: &[i64], tag_id: i64) -> Result<u64> {
        if asset_ids.is_empty() {
            return Ok(0);
        }
        let mut updated = 0u64;
        for asset_id in asset_ids {
            let result =
                sqlx::query("INSERT OR IGNORE INTO asset_tag (asset_id, tag_id) VALUES (?, ?)")
                    .bind(asset_id)
                    .bind(tag_id)
                    .execute(self.write())
                    .await?;
            if result.rows_affected() > 0 {
                updated += 1;
            }
        }
        Ok(updated)
    }

    pub async fn remove_tag_id_from_assets(&self, asset_ids: &[i64], tag_id: i64) -> Result<u64> {
        if asset_ids.is_empty() {
            return Ok(0);
        }
        let mut updated = 0u64;
        for asset_id in asset_ids {
            let result = sqlx::query("DELETE FROM asset_tag WHERE asset_id = ? AND tag_id = ?")
                .bind(asset_id)
                .bind(tag_id)
                .execute(self.write())
                .await?;
            if result.rows_affected() > 0 {
                updated += 1;
            }
        }
        Ok(updated)
    }

    pub async fn create_tag(
        &self,
        name: &str,
        parent_id: Option<i64>,
        color: Option<&str>,
    ) -> Result<i64> {
        if let Some(parent_id) = parent_id {
            let exists = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM tag WHERE id = ?")
                .bind(parent_id)
                .fetch_one(self.write())
                .await?;
            if exists == 0 {
                return Err(AppError::NotFound(format!("parent tag {}", parent_id)));
            }
        }

        sqlx::query("INSERT OR IGNORE INTO tag (name, parent_id, color) VALUES (?, ?, ?)")
            .bind(name)
            .bind(parent_id)
            .bind(color)
            .execute(self.write())
            .await?;
        if color.is_some() {
            sqlx::query("UPDATE tag SET color = ? WHERE name = ?")
                .bind(color)
                .bind(name)
                .execute(self.write())
                .await?;
        }
        sqlx::query_scalar::<_, i64>("SELECT id FROM tag WHERE name = ?")
            .bind(name)
            .fetch_one(self.write())
            .await
            .map_err(|e| AppError::Catalog(e.to_string()))
    }

    pub async fn update_tag(&self, id: i64, name: &str, color: Option<&str>) -> Result<TagRow> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(AppError::InvalidInput("tag name is required".into()));
        }
        let result = sqlx::query("UPDATE tag SET name = ?, color = ? WHERE id = ?")
            .bind(trimmed)
            .bind(color)
            .bind(id)
            .execute(self.write())
            .await?;
        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("tag {}", id)));
        }
        self.fetch_tag_row(id).await
    }

    pub async fn delete_tag(&self, id: i64) -> Result<()> {
        let child_count =
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM tag WHERE parent_id = ?")
                .bind(id)
                .fetch_one(self.write())
                .await?;
        if child_count > 0 {
            return Err(AppError::InvalidInput(
                "cannot delete tag with subtags".into(),
            ));
        }

        sqlx::query("DELETE FROM asset_tag WHERE tag_id = ?")
            .bind(id)
            .execute(self.write())
            .await?;

        let result = sqlx::query("DELETE FROM tag WHERE id = ?")
            .bind(id)
            .execute(self.write())
            .await?;
        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("tag {}", id)));
        }
        Ok(())
    }

    async fn fetch_tag_row(&self, id: i64) -> Result<TagRow> {
        let sql = format!(
            r#"
            SELECT
              t.id,
              t.name,
              t.parent_id,
              t.color,
              {asset_count}
            FROM tag t
            WHERE t.id = ?
            "#,
            asset_count = TAG_ASSET_COUNT_SQL,
        );
        sqlx::query_as::<_, TagRow>(&sql)
            .bind(id)
            .fetch_one(self.write())
            .await
            .map_err(|e| AppError::Catalog(e.to_string()))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::repo::test_support::test_catalog;
    use crate::catalog::repo::{AssetRepo, SourceRootRepo, UpsertAssetInput};

    #[tokio::test]
    async fn delete_tag_removes_asset_links() {
        let (catalog, _dir) = test_catalog().await;
        let roots = SourceRootRepo::new(catalog.pools().clone());
        let assets = AssetRepo::new(catalog.pools().clone());
        let tag_repo = TagRepo::new(catalog.pools().clone());

        let root = roots
            .insert_root("/tmp/p", "local", "watch", None)
            .await
            .unwrap();
        let asset = assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "d.jpg",
                file_name: "d.jpg",
                ext: "jpg",
                kind: "image",
                size: 1,
                mtime_ns: 1,
                sync_state: "ok",
            })
            .await
            .unwrap();

        let tag_id = tag_repo
            .create_tag("travel", None, Some("#ff0000"))
            .await
            .unwrap();
        tag_repo
            .append_tag_id_to_assets(&[asset.id], tag_id)
            .await
            .unwrap();

        tag_repo.delete_tag(tag_id).await.unwrap();

        let tag_ids = tag_repo.list_ids_for_asset(asset.id).await.unwrap();
        assert!(tag_ids.is_empty());
        let tags = tag_repo.list_tag_rows().await.unwrap();
        assert!(tags.iter().all(|tag| tag.id != tag_id));
    }

    #[tokio::test]
    async fn expand_tag_ids_includes_descendants() {
        let (catalog, _dir) = test_catalog().await;
        let tag_repo = TagRepo::new(catalog.pools().clone());

        let parent_id = tag_repo.create_tag("travel", None, None).await.unwrap();
        let child_id = tag_repo
            .create_tag("japan", Some(parent_id), None)
            .await
            .unwrap();

        let expanded = tag_repo
            .expand_tag_ids_with_descendants(&[parent_id])
            .await
            .unwrap();
        assert_eq!(expanded, vec![parent_id, child_id]);
    }

    #[tokio::test]
    async fn parent_tag_asset_count_includes_descendants() {
        let (catalog, _dir) = test_catalog().await;
        let roots = SourceRootRepo::new(catalog.pools().clone());
        let assets = AssetRepo::new(catalog.pools().clone());
        let tag_repo = TagRepo::new(catalog.pools().clone());

        let root = roots
            .insert_root("/tmp/p", "local", "watch", None)
            .await
            .unwrap();
        let asset = assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "e.jpg",
                file_name: "e.jpg",
                ext: "jpg",
                kind: "image",
                size: 1,
                mtime_ns: 1,
                sync_state: "ok",
            })
            .await
            .unwrap();

        let parent_id = tag_repo.create_tag("places", None, None).await.unwrap();
        let child_id = tag_repo
            .create_tag("tokyo", Some(parent_id), None)
            .await
            .unwrap();
        tag_repo
            .append_tag_id_to_assets(&[asset.id], child_id)
            .await
            .unwrap();

        let rows = tag_repo.list_tag_rows().await.unwrap();
        let parent = rows.iter().find(|row| row.id == parent_id).unwrap();
        let child = rows.iter().find(|row| row.id == child_id).unwrap();
        assert_eq!(parent.asset_count, 1);
        assert_eq!(child.asset_count, 1);
    }

    #[tokio::test]
    async fn tag_repo_empty_inputs_and_validation_errors() {
        let (catalog, _dir) = test_catalog().await;
        let tag_repo = TagRepo::new(catalog.pools().clone());

        assert!(tag_repo
            .expand_tag_ids_with_descendants(&[])
            .await
            .unwrap()
            .is_empty());
        assert_eq!(tag_repo.append_tag_id_to_assets(&[], 1).await.unwrap(), 0);
        assert_eq!(tag_repo.remove_tag_id_from_assets(&[], 1).await.unwrap(), 0);

        let err = tag_repo
            .create_tag("child", Some(999), None)
            .await
            .unwrap_err();
        assert!(err.to_string().contains("not found"));

        let tag_id = tag_repo.create_tag("rename-me", None, None).await.unwrap();
        let err = tag_repo.update_tag(tag_id, "   ", None).await.unwrap_err();
        assert!(err.to_string().contains("required"));
        let err = tag_repo.update_tag(999, "missing", None).await.unwrap_err();
        assert!(err.to_string().contains("not found"));

        let parent_id = tag_repo.create_tag("parent", None, None).await.unwrap();
        tag_repo
            .create_tag("child-tag", Some(parent_id), None)
            .await
            .unwrap();
        let err = tag_repo.delete_tag(parent_id).await.unwrap_err();
        assert!(err.to_string().contains("subtags"));

        let err = tag_repo.delete_tag(999).await.unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[tokio::test]
    async fn tag_listings_remove_update_and_duplicate_append() {
        let (catalog, _dir) = test_catalog().await;
        let roots = SourceRootRepo::new(catalog.pools().clone());
        let assets = AssetRepo::new(catalog.pools().clone());
        let tag_repo = TagRepo::new(catalog.pools().clone());
        let root = roots
            .insert_root("/tmp/p", "local", "watch", None)
            .await
            .unwrap();
        let asset = assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "tagged.jpg",
                file_name: "tagged.jpg",
                ext: "jpg",
                kind: "image",
                size: 1,
                mtime_ns: 1,
                sync_state: "ok",
            })
            .await
            .unwrap();
        let parent_id = tag_repo.create_tag("places", None, None).await.unwrap();
        let child_id = tag_repo
            .create_tag("tokyo", Some(parent_id), None)
            .await
            .unwrap();
        tag_repo
            .append_tag_id_to_assets(&[asset.id], child_id)
            .await
            .unwrap();
        assert_eq!(
            tag_repo
                .append_tag_id_to_assets(&[asset.id], child_id)
                .await
                .unwrap(),
            0
        );
        let names = tag_repo.list_names_for_asset(asset.id).await.unwrap();
        assert_eq!(names, vec!["tokyo".to_string()]);
        let all_names = tag_repo.list_all_names().await.unwrap();
        assert!(all_names.contains(&"places".to_string()));
        let asset_ids = tag_repo.list_asset_ids_for_tag(child_id).await.unwrap();
        assert_eq!(asset_ids, vec![asset.id]);
        let descendants = tag_repo.descendant_tag_ids(parent_id).await.unwrap();
        assert_eq!(descendants, vec![parent_id, child_id]);
        let updated = tag_repo
            .update_tag(child_id, "osaka", Some("#00ff00"))
            .await
            .unwrap();
        assert_eq!(updated.name, "osaka");
        assert_eq!(
            tag_repo
                .remove_tag_id_from_assets(&[asset.id], child_id)
                .await
                .unwrap(),
            1
        );
        assert!(tag_repo
            .list_names_for_asset(asset.id)
            .await
            .unwrap()
            .is_empty());
    }

    #[tokio::test]
    async fn create_tag_existing_name_reuses_id_and_sets_color() {
        let (catalog, _dir) = test_catalog().await;
        let tag_repo = TagRepo::new(catalog.pools().clone());
        let first_id = tag_repo.create_tag("shared", None, None).await.unwrap();
        let second_id = tag_repo
            .create_tag("shared", None, Some("#aabbcc"))
            .await
            .unwrap();
        assert_eq!(first_id, second_id);
        let rows = tag_repo.list_tag_rows().await.unwrap();
        let tag = rows.iter().find(|row| row.id == first_id).unwrap();
        assert_eq!(tag.color.as_deref(), Some("#aabbcc"));
    }

    #[tokio::test]
    async fn remove_tag_id_from_assets_returns_zero_when_unassigned() {
        let (catalog, _dir) = test_catalog().await;
        let roots = SourceRootRepo::new(catalog.pools().clone());
        let assets = AssetRepo::new(catalog.pools().clone());
        let tag_repo = TagRepo::new(catalog.pools().clone());
        let root = roots
            .insert_root("/tmp/p", "local", "watch", None)
            .await
            .unwrap();
        let asset = assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "plain.jpg",
                file_name: "plain.jpg",
                ext: "jpg",
                kind: "image",
                size: 1,
                mtime_ns: 1,
                sync_state: "ok",
            })
            .await
            .unwrap();
        let tag_id = tag_repo.create_tag("solo", None, None).await.unwrap();
        assert_eq!(
            tag_repo
                .remove_tag_id_from_assets(&[asset.id], tag_id)
                .await
                .unwrap(),
            0
        );
    }

    #[tokio::test]
    async fn update_tag_fetch_errors_when_asset_table_is_dropped() {
        let (catalog, _dir) = test_catalog().await;
        let tag_repo = TagRepo::new(catalog.pools().clone());
        let tag_id = tag_repo.create_tag("rename", None, None).await.unwrap();
        sqlx::query("DROP TABLE asset")
            .execute(catalog.write_pool())
            .await
            .unwrap();
        let err = tag_repo
            .update_tag(tag_id, "renamed", None)
            .await
            .unwrap_err();
        assert!(!err.to_string().is_empty());
    }

    #[tokio::test]
    async fn tag_mutations_error_under_exclusive_lock() {
        let (catalog, _dir) = test_catalog().await;
        let pools = catalog.pools().clone();
        let tag_repo = TagRepo::new(pools.clone());
        let parent_id = tag_repo.create_tag("parent", None, None).await.unwrap();
        let tag_id = tag_repo.create_tag("leaf", None, None).await.unwrap();
        let mut locker = pools.write().acquire().await.unwrap();
        sqlx::query("BEGIN EXCLUSIVE")
            .execute(&mut *locker)
            .await
            .unwrap();
        assert!(tag_repo
            .create_tag("blocked", Some(parent_id), Some("#fff"))
            .await
            .is_err());
        assert!(tag_repo.delete_tag(tag_id).await.is_err());
        sqlx::query("ROLLBACK").execute(&mut *locker).await.unwrap();
    }

    #[tokio::test]
    async fn tag_repo_errors_after_pool_close() {
        let (catalog, _dir) = test_catalog().await;
        let pools = catalog.pools().clone();
        let roots = SourceRootRepo::new(pools.clone());
        let assets = AssetRepo::new(pools.clone());
        let tag_repo = TagRepo::new(pools.clone());
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
        let tag_id = tag_repo.create_tag("travel", None, None).await.unwrap();
        tag_repo
            .append_tag_id_to_assets(&[asset.id], tag_id)
            .await
            .unwrap();
        pools.close().await;
        assert!(tag_repo.list_ids_for_asset(asset.id).await.is_err());
        assert!(tag_repo.list_names_for_asset(asset.id).await.is_err());
        assert!(tag_repo.list_all_names().await.is_err());
        assert!(tag_repo.list_asset_ids_for_tag(tag_id).await.is_err());
        assert!(tag_repo.list_tag_rows().await.is_err());
        assert!(tag_repo.descendant_tag_ids(tag_id).await.is_err());
        assert!(tag_repo
            .expand_tag_ids_with_descendants(&[tag_id])
            .await
            .is_err());
        assert!(tag_repo
            .append_tag_id_to_assets(&[asset.id], tag_id)
            .await
            .is_err());
        assert!(tag_repo
            .remove_tag_id_from_assets(&[asset.id], tag_id)
            .await
            .is_err());
        assert!(tag_repo.create_tag("new", None, None).await.is_err());
        assert!(tag_repo.update_tag(tag_id, "new", None).await.is_err());
        assert!(tag_repo.delete_tag(tag_id).await.is_err());
    }

    #[tokio::test]
    async fn create_tag_rejects_missing_parent() {
        let (catalog, _dir) = test_catalog().await;
        let tag_repo = TagRepo::new(catalog.pools().clone());
        let err = tag_repo
            .create_tag("child", Some(999_999), None)
            .await
            .unwrap_err();
        assert!(err.to_string().contains("parent tag"));
    }

    #[tokio::test]
    async fn create_tag_updates_color_for_existing_name() {
        let (catalog, _dir) = test_catalog().await;
        let tag_repo = TagRepo::new(catalog.pools().clone());
        let first = tag_repo
            .create_tag("colorful", None, Some("#111111"))
            .await
            .unwrap();
        let second = tag_repo
            .create_tag("colorful", None, Some("#222222"))
            .await
            .unwrap();
        assert_eq!(first, second);
        let rows = tag_repo.list_tag_rows().await.unwrap();
        let row = rows.iter().find(|row| row.id == first).unwrap();
        assert_eq!(row.color.as_deref(), Some("#222222"));
    }

    #[tokio::test]
    async fn delete_tag_returns_not_found_for_missing_id() {
        let (catalog, _dir) = test_catalog().await;
        let tag_repo = TagRepo::new(catalog.pools().clone());
        let err = tag_repo.delete_tag(999_999).await.unwrap_err();
        assert!(err.to_string().contains("tag"));
    }

    #[tokio::test]
    async fn create_tag_fetch_errors_under_exclusive_lock() {
        let (catalog, _dir) = test_catalog().await;
        let pools = catalog.pools().clone();
        let tag_repo = TagRepo::new(pools.clone());
        let mut locker = pools.write().acquire().await.unwrap();
        sqlx::query("BEGIN EXCLUSIVE")
            .execute(&mut *locker)
            .await
            .unwrap();
        assert!(tag_repo
            .create_tag("locked", None, Some("#abc"))
            .await
            .is_err());
        sqlx::query("ROLLBACK").execute(&mut *locker).await.unwrap();
    }
}
