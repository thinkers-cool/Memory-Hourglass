use crate::catalog::models::{Asset, AssetScanState};
use crate::error::{AppError, Result};
use sqlx::SqlitePool;

pub struct UpsertAssetInput<'a> {
    pub root_id: i64,
    pub rel_path: &'a str,
    pub file_name: &'a str,
    pub ext: &'a str,
    pub kind: &'a str,
    pub size: i64,
    pub mtime_ns: i64,
    pub sync_state: &'a str,
}

pub struct AssetRepo {
    pub(crate) pool: SqlitePool,
}

impl AssetRepo {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn upsert_asset(&self, input: UpsertAssetInput<'_>) -> Result<Asset> {
        let UpsertAssetInput {
            root_id,
            rel_path,
            file_name,
            ext,
            kind,
            size,
            mtime_ns,
            sync_state,
        } = input;
        let id = sqlx::query_scalar::<_, i64>(
            r#"
            INSERT INTO asset (root_id, rel_path, file_name, ext, kind, size, mtime_ns, sync_state, deleted_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, NULL)
            ON CONFLICT(root_id, rel_path) DO UPDATE SET
                file_name = excluded.file_name,
                ext = excluded.ext,
                kind = excluded.kind,
                size = excluded.size,
                mtime_ns = excluded.mtime_ns,
                sync_state = excluded.sync_state
            WHERE deleted_at IS NULL
            RETURNING id
            "#,
        )
        .bind(root_id)
        .bind(rel_path)
        .bind(file_name)
        .bind(ext)
        .bind(kind)
        .bind(size)
        .bind(mtime_ns)
        .bind(sync_state)
        .fetch_one(&self.pool)
        .await?;

        self.get_asset(id).await
    }

    pub async fn upsert_assets_batch(
        &self,
        root_id: i64,
        inputs: &[UpsertAssetInput<'_>],
    ) -> Result<Vec<i64>> {
        if inputs.is_empty() {
            return Ok(Vec::new());
        }

        let mut tx = self.pool.begin().await?;
        let mut ids = Vec::with_capacity(inputs.len());
        for input in inputs {
            let id = sqlx::query_scalar::<_, i64>(
                r#"
                INSERT INTO asset (root_id, rel_path, file_name, ext, kind, size, mtime_ns, sync_state, deleted_at)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, NULL)
                ON CONFLICT(root_id, rel_path) DO UPDATE SET
                    file_name = excluded.file_name,
                    ext = excluded.ext,
                    kind = excluded.kind,
                    size = excluded.size,
                    mtime_ns = excluded.mtime_ns,
                    sync_state = excluded.sync_state
                WHERE deleted_at IS NULL
                RETURNING id
                "#,
            )
            .bind(root_id)
            .bind(input.rel_path)
            .bind(input.file_name)
            .bind(input.ext)
            .bind(input.kind)
            .bind(input.size)
            .bind(input.mtime_ns)
            .bind(input.sync_state)
            .fetch_one(&mut *tx)
            .await?;
            ids.push(id);
        }
        tx.commit().await?;
        Ok(ids)
    }

    pub async fn get_asset(&self, id: i64) -> Result<Asset> {
        sqlx::query_as::<_, Asset>("SELECT * FROM asset WHERE id = ? AND deleted_at IS NULL")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("asset {}", id)))
    }

    pub async fn get_assets_by_ids(&self, ids: &[i64]) -> Result<Vec<Asset>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut builder =
            sqlx::QueryBuilder::new("SELECT * FROM asset WHERE deleted_at IS NULL AND id IN (");
        let mut separated = builder.separated(", ");
        for id in ids {
            separated.push_bind(id);
        }
        separated.push_unseparated(")");
        Ok(builder
            .build_query_as::<Asset>()
            .fetch_all(&self.pool)
            .await?)
    }

    pub async fn find_by_path(&self, root_id: i64, rel_path: &str) -> Result<Option<Asset>> {
        Ok(sqlx::query_as::<_, Asset>(
            "SELECT * FROM asset WHERE root_id = ? AND rel_path = ? AND deleted_at IS NULL",
        )
        .bind(root_id)
        .bind(rel_path)
        .fetch_optional(&self.pool)
        .await?)
    }

    pub async fn list_paths_for_root(&self, root_id: i64) -> Result<Vec<(String, i64, i64)>> {
        let rows = sqlx::query_as::<_, (String, i64, i64)>(
            "SELECT rel_path, mtime_ns, size FROM asset WHERE root_id = ? AND deleted_at IS NULL",
        )
        .bind(root_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn list_scan_state_for_root(&self, root_id: i64) -> Result<Vec<AssetScanState>> {
        Ok(sqlx::query_as::<_, AssetScanState>(
            r#"
                SELECT
                    rel_path,
                    mtime_ns,
                    size,
                    indexed_mtime_ns,
                    thumb_key,
                    kind,
                    (
                        SELECT COUNT(*)
                        FROM asset_raw_tag rt
                        WHERE rt.asset_id = asset.id
                    ) AS raw_tag_count
                FROM asset
                WHERE root_id = ? AND deleted_at IS NULL
                "#,
        )
        .bind(root_id)
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn list_deleted_paths_for_root(&self, root_id: i64) -> Result<Vec<String>> {
        Ok(sqlx::query_scalar::<_, String>(
            "SELECT rel_path FROM asset WHERE root_id = ? AND deleted_at IS NOT NULL",
        )
        .bind(root_id)
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn find_by_path_including_deleted(
        &self,
        root_id: i64,
        rel_path: &str,
    ) -> Result<Option<Asset>> {
        Ok(
            sqlx::query_as::<_, Asset>("SELECT * FROM asset WHERE root_id = ? AND rel_path = ?")
                .bind(root_id)
                .bind(rel_path)
                .fetch_optional(&self.pool)
                .await?,
        )
    }

    pub async fn count_for_root(&self, root_id: i64) -> Result<(i64, i64)> {
        let total = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM asset WHERE root_id = ? AND deleted_at IS NULL",
        )
        .bind(root_id)
        .fetch_one(&self.pool)
        .await?;
        let missing = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM asset WHERE root_id = ? AND deleted_at IS NULL AND sync_state = 'missing'",
        )
        .bind(root_id)
        .fetch_one(&self.pool)
        .await?;
        Ok((total, missing))
    }

    pub async fn mark_missing(&self, root_id: i64, rel_paths: &[String]) -> Result<()> {
        if rel_paths.is_empty() {
            return Ok(());
        }
        let mut builder =
            sqlx::QueryBuilder::new("UPDATE asset SET sync_state = 'missing' WHERE root_id = ");
        builder.push_bind(root_id);
        builder.push(" AND rel_path IN (");
        for (index, rel) in rel_paths.iter().enumerate() {
            if index > 0 {
                builder.push(", ");
            }
            builder.push_bind(rel);
        }
        builder.push(")");
        builder.build().execute(&self.pool).await?;
        Ok(())
    }

    pub async fn set_thumb_key(&self, asset_id: i64, thumb_key: &str) -> Result<()> {
        sqlx::query("UPDATE asset SET thumb_key = ? WHERE id = ?")
            .bind(thumb_key)
            .bind(asset_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn update_file_stats(&self, asset_id: i64, mtime_ns: i64, size: i64) -> Result<()> {
        sqlx::query("UPDATE asset SET mtime_ns = ?, size = ? WHERE id = ? AND deleted_at IS NULL")
            .bind(mtime_ns)
            .bind(size)
            .bind(asset_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn mark_indexed(&self, asset_id: i64, mtime_ns: i64) -> Result<()> {
        sqlx::query(
            "UPDATE asset SET indexed_mtime_ns = ?, sync_state = 'ok' WHERE id = ? AND deleted_at IS NULL",
        )
        .bind(mtime_ns)
        .bind(asset_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn set_sync_state(&self, asset_id: i64, sync_state: &str) -> Result<()> {
        sqlx::query("UPDATE asset SET sync_state = ? WHERE id = ? AND deleted_at IS NULL")
            .bind(sync_state)
            .bind(asset_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn soft_delete(&self, ids: &[i64], at: i64) -> Result<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        let mut builder = sqlx::QueryBuilder::new("UPDATE asset SET deleted_at = ");
        builder.push_bind(at);
        builder.push(", sync_state = 'ok' WHERE deleted_at IS NULL AND id IN (");
        for (index, id) in ids.iter().enumerate() {
            if index > 0 {
                builder.push(", ");
            }
            builder.push_bind(*id);
        }
        builder.push(")");
        let result = builder.build().execute(&self.pool).await?;
        Ok(result.rows_affected())
    }

    pub async fn get_deleted_at_map(
        &self,
        ids: &[i64],
    ) -> Result<std::collections::HashMap<i64, i64>> {
        if ids.is_empty() {
            return Ok(std::collections::HashMap::new());
        }
        let mut builder = sqlx::QueryBuilder::new(
            "SELECT id, deleted_at FROM asset WHERE deleted_at IS NOT NULL AND id IN (",
        );
        for (index, id) in ids.iter().enumerate() {
            if index > 0 {
                builder.push(", ");
            }
            builder.push_bind(*id);
        }
        builder.push(")");
        let rows = builder
            .build_query_as::<(i64, i64)>()
            .fetch_all(&self.pool)
            .await?;
        Ok(rows.into_iter().collect())
    }

    pub async fn restore_assets(&self, ids: &[i64]) -> Result<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        let mut builder = sqlx::QueryBuilder::new(
            "UPDATE asset SET deleted_at = NULL WHERE deleted_at IS NOT NULL AND id IN (",
        );
        for (index, id) in ids.iter().enumerate() {
            if index > 0 {
                builder.push(", ");
            }
            builder.push_bind(*id);
        }
        builder.push(")");
        let result = builder.build().execute(&self.pool).await?;
        Ok(result.rows_affected())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::repo::test_support::test_catalog;
    use crate::catalog::repo::SourceRootRepo;

    #[tokio::test]
    async fn upsert_asset_creates_and_updates() {
        let (catalog, _dir) = test_catalog().await;
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let assets = AssetRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root("/tmp/p", "local", "watch", None)
            .await
            .unwrap();

        let a1 = assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "a.jpg",
                file_name: "a.jpg",
                ext: "jpg",
                kind: "image",
                size: 100,
                mtime_ns: 1,
                sync_state: "new",
            })
            .await
            .unwrap();
        assert_eq!(a1.sync_state, "new");

        let a2 = assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "a.jpg",
                file_name: "a.jpg",
                ext: "jpg",
                kind: "image",
                size: 200,
                mtime_ns: 2,
                sync_state: "modified",
            })
            .await
            .unwrap();
        assert_eq!(a2.id, a1.id);
        assert_eq!(a2.size, 200);
        assert_eq!(a2.sync_state, "modified");
    }

    #[tokio::test]
    async fn soft_delete_hides_asset() {
        let (catalog, _dir) = test_catalog().await;
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let assets = AssetRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root("/tmp/p", "local", "watch", None)
            .await
            .unwrap();
        let asset = assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "b.jpg",
                file_name: "b.jpg",
                ext: "jpg",
                kind: "image",
                size: 1,
                mtime_ns: 1,
                sync_state: "ok",
            })
            .await
            .unwrap();

        assets.soft_delete(&[asset.id], 99).await.unwrap();
        let err = assets.get_asset(asset.id).await.unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[tokio::test]
    async fn asset_not_found_and_empty_batch_helpers() {
        let (catalog, _dir) = test_catalog().await;
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let assets = AssetRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root("/tmp/p", "local", "watch", None)
            .await
            .unwrap();

        let err = assets.get_asset(999).await.unwrap_err();
        assert!(err.to_string().contains("not found"));
        assert!(assets
            .upsert_assets_batch(root.id, &[])
            .await
            .unwrap()
            .is_empty());
        assert_eq!(assets.soft_delete(&[], 1).await.unwrap(), 0);
        assert!(assets.get_deleted_at_map(&[]).await.unwrap().is_empty());
        assert_eq!(assets.restore_assets(&[]).await.unwrap(), 0);
        assets.mark_missing(root.id, &[]).await.unwrap();
    }

    #[tokio::test]
    async fn mark_missing_updates_multiple_assets() {
        let (catalog, _dir) = test_catalog().await;
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let assets = AssetRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root("/tmp/p", "local", "watch", None)
            .await
            .unwrap();
        for name in ["one.jpg", "two.jpg"] {
            assets
                .upsert_asset(UpsertAssetInput {
                    root_id: root.id,
                    rel_path: name,
                    file_name: name,
                    ext: "jpg",
                    kind: "image",
                    size: 1,
                    mtime_ns: 1,
                    sync_state: "ok",
                })
                .await
                .unwrap();
        }
        assets
            .mark_missing(root.id, &["one.jpg".into(), "two.jpg".into()])
            .await
            .unwrap();
        let (_, missing) = assets.count_for_root(root.id).await.unwrap();
        assert_eq!(missing, 2);
    }

    #[tokio::test]
    async fn soft_delete_and_restore_multiple_assets() {
        let (catalog, _dir) = test_catalog().await;
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let assets = AssetRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root("/tmp/p", "local", "watch", None)
            .await
            .unwrap();
        let mut ids = Vec::new();
        for name in ["a.jpg", "b.jpg"] {
            let asset = assets
                .upsert_asset(UpsertAssetInput {
                    root_id: root.id,
                    rel_path: name,
                    file_name: name,
                    ext: "jpg",
                    kind: "image",
                    size: 1,
                    mtime_ns: 1,
                    sync_state: "ok",
                })
                .await
                .unwrap();
            ids.push(asset.id);
        }
        assert_eq!(assets.soft_delete(&ids, 42).await.unwrap(), 2);
        let deleted = assets.get_deleted_at_map(&ids).await.unwrap();
        assert_eq!(deleted.len(), 2);
        assert_eq!(assets.restore_assets(&ids).await.unwrap(), 2);
    }

    #[tokio::test]
    async fn upsert_asset_errors_when_path_soft_deleted() {
        let (catalog, _dir) = test_catalog().await;
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let assets = AssetRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root("/tmp/p", "local", "watch", None)
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
        assets.soft_delete(&[asset.id], 1).await.unwrap();
        assert!(assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "gone.jpg",
                file_name: "gone.jpg",
                ext: "jpg",
                kind: "image",
                size: 2,
                mtime_ns: 2,
                sync_state: "new",
            })
            .await
            .is_err());
    }

    #[tokio::test]
    async fn find_by_path_and_deleted_lookup_behaviors() {
        let (catalog, _dir) = test_catalog().await;
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let assets = AssetRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root("/tmp/p", "local", "watch", None)
            .await
            .unwrap();
        let asset = assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "live.jpg",
                file_name: "live.jpg",
                ext: "jpg",
                kind: "image",
                size: 1,
                mtime_ns: 1,
                sync_state: "ok",
            })
            .await
            .unwrap();
        assert!(assets
            .find_by_path(root.id, "missing.jpg")
            .await
            .unwrap()
            .is_none());
        assets.soft_delete(&[asset.id], 9).await.unwrap();
        assert!(assets
            .find_by_path(root.id, "live.jpg")
            .await
            .unwrap()
            .is_none());
        let deleted = assets
            .find_by_path_including_deleted(root.id, "live.jpg")
            .await
            .unwrap()
            .unwrap();
        assert!(deleted.deleted_at.is_some());
    }

    #[tokio::test]
    async fn list_paths_and_deleted_paths_filter_soft_deleted() {
        let (catalog, _dir) = test_catalog().await;
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let assets = AssetRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root("/tmp/p", "local", "watch", None)
            .await
            .unwrap();
        let live = assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "live.jpg",
                file_name: "live.jpg",
                ext: "jpg",
                kind: "image",
                size: 1,
                mtime_ns: 1,
                sync_state: "ok",
            })
            .await
            .unwrap();
        let gone = assets
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
        assets.soft_delete(&[gone.id], 1).await.unwrap();
        let paths = assets.list_paths_for_root(root.id).await.unwrap();
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].0, "live.jpg");
        let deleted_paths = assets.list_deleted_paths_for_root(root.id).await.unwrap();
        assert_eq!(deleted_paths, vec!["gone.jpg".to_string()]);
        let _ = live;
    }

    #[tokio::test]
    async fn count_for_root_and_mark_missing_unknown_paths() {
        let (catalog, _dir) = test_catalog().await;
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let assets = AssetRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root("/tmp/p", "local", "watch", None)
            .await
            .unwrap();
        assets
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
        assets
            .mark_missing(root.id, &["ghost.jpg".into()])
            .await
            .unwrap();
        let (total, missing) = assets.count_for_root(root.id).await.unwrap();
        assert_eq!(total, 1);
        assert_eq!(missing, 0);
    }

    #[tokio::test]
    async fn mutators_skip_soft_deleted_assets() {
        let (catalog, _dir) = test_catalog().await;
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let assets = AssetRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root("/tmp/p", "local", "watch", None)
            .await
            .unwrap();
        let asset = assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "gone.jpg",
                file_name: "gone.jpg",
                ext: "jpg",
                kind: "image",
                size: 10,
                mtime_ns: 10,
                sync_state: "new",
            })
            .await
            .unwrap();
        assets.soft_delete(&[asset.id], 1).await.unwrap();
        assets.update_file_stats(asset.id, 20, 20).await.unwrap();
        assets.mark_indexed(asset.id, 20).await.unwrap();
        assets.set_sync_state(asset.id, "missing").await.unwrap();
        let row = assets
            .find_by_path_including_deleted(root.id, "gone.jpg")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(row.size, 10);
        assert_eq!(row.sync_state, "ok");
        assert!(row.indexed_mtime_ns.is_none());
    }

    #[tokio::test]
    async fn set_thumb_key_and_batch_upsert_roundtrip() {
        let (catalog, _dir) = test_catalog().await;
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let assets = AssetRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root("/tmp/p", "local", "watch", None)
            .await
            .unwrap();
        let ids = assets
            .upsert_assets_batch(
                root.id,
                &[
                    UpsertAssetInput {
                        root_id: root.id,
                        rel_path: "a.jpg",
                        file_name: "a.jpg",
                        ext: "jpg",
                        kind: "image",
                        size: 1,
                        mtime_ns: 1,
                        sync_state: "new",
                    },
                    UpsertAssetInput {
                        root_id: root.id,
                        rel_path: "b.jpg",
                        file_name: "b.jpg",
                        ext: "jpg",
                        kind: "image",
                        size: 1,
                        mtime_ns: 1,
                        sync_state: "new",
                    },
                ],
            )
            .await
            .unwrap();
        assert_eq!(ids.len(), 2);
        assets.set_thumb_key(ids[0], "1.webp").await.unwrap();
        let asset = assets.get_asset(ids[0]).await.unwrap();
        assert_eq!(asset.thumb_key.as_deref(), Some("1.webp"));
    }

    #[tokio::test]
    async fn live_asset_mutators_and_find_by_path_success() {
        let (catalog, _dir) = test_catalog().await;
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let assets = AssetRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root("/tmp/p", "local", "watch", None)
            .await
            .unwrap();
        let asset = assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "live.jpg",
                file_name: "live.jpg",
                ext: "jpg",
                kind: "image",
                size: 10,
                mtime_ns: 10,
                sync_state: "new",
            })
            .await
            .unwrap();
        let found = assets
            .find_by_path(root.id, "live.jpg")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(found.id, asset.id);
        assets.update_file_stats(asset.id, 20, 30).await.unwrap();
        assets.mark_indexed(asset.id, 20).await.unwrap();
        assets.set_sync_state(asset.id, "ok").await.unwrap();
        let updated = assets.get_asset(asset.id).await.unwrap();
        assert_eq!(updated.size, 30);
        assert_eq!(updated.sync_state, "ok");
        assert_eq!(updated.indexed_mtime_ns, Some(20));
    }

    #[tokio::test]
    async fn asset_repo_errors_after_pool_close() {
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
        pool.close().await;
        assert!(assets.get_asset(asset.id).await.is_err());
        assert!(assets.find_by_path(root.id, "a.jpg").await.is_err());
        assert!(assets.list_paths_for_root(root.id).await.is_err());
        assert!(assets.list_scan_state_for_root(root.id).await.is_err());
        assert!(assets.list_deleted_paths_for_root(root.id).await.is_err());
        assert!(assets
            .find_by_path_including_deleted(root.id, "a.jpg")
            .await
            .is_err());
        assert!(assets.count_for_root(root.id).await.is_err());
        assert!(assets
            .mark_missing(root.id, &["a.jpg".into()])
            .await
            .is_err());
        assert!(assets.set_thumb_key(asset.id, "t").await.is_err());
        assert!(assets.update_file_stats(asset.id, 1, 1).await.is_err());
        assert!(assets.mark_indexed(asset.id, 1).await.is_err());
        assert!(assets.set_sync_state(asset.id, "ok").await.is_err());
        assert!(assets.soft_delete(&[asset.id], 1).await.is_err());
        assert!(assets.get_deleted_at_map(&[asset.id]).await.is_err());
        assert!(assets.restore_assets(&[asset.id]).await.is_err());
        assert!(assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "b.jpg",
                file_name: "b.jpg",
                ext: "jpg",
                kind: "image",
                size: 1,
                mtime_ns: 1,
                sync_state: "ok",
            })
            .await
            .is_err());
        assert!(assets
            .upsert_assets_batch(
                root.id,
                &[UpsertAssetInput {
                    root_id: root.id,
                    rel_path: "c.jpg",
                    file_name: "c.jpg",
                    ext: "jpg",
                    kind: "image",
                    size: 1,
                    mtime_ns: 1,
                    sync_state: "ok",
                }],
            )
            .await
            .is_err());
    }

    #[tokio::test]
    async fn upsert_assets_batch_errors_when_path_soft_deleted() {
        let (catalog, _dir) = test_catalog().await;
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let assets = AssetRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root("/tmp/p", "local", "watch", None)
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
        assets.soft_delete(&[asset.id], 1).await.unwrap();
        assert!(assets
            .upsert_assets_batch(
                root.id,
                &[UpsertAssetInput {
                    root_id: root.id,
                    rel_path: "gone.jpg",
                    file_name: "gone.jpg",
                    ext: "jpg",
                    kind: "image",
                    size: 2,
                    mtime_ns: 2,
                    sync_state: "new",
                }],
            )
            .await
            .is_err());
    }

    #[tokio::test]
    async fn upsert_assets_batch_errors_under_exclusive_lock() {
        let (catalog, _dir) = test_catalog().await;
        let pool = catalog.pool().clone();
        let roots = SourceRootRepo::new(pool.clone());
        let assets = AssetRepo::new(pool.clone());
        let root = roots
            .insert_root("/tmp/p", "local", "watch", None)
            .await
            .unwrap();
        let mut locker = pool.acquire().await.unwrap();
        sqlx::query("BEGIN EXCLUSIVE")
            .execute(&mut *locker)
            .await
            .unwrap();
        assert!(assets
            .upsert_assets_batch(
                root.id,
                &[UpsertAssetInput {
                    root_id: root.id,
                    rel_path: "lock.jpg",
                    file_name: "lock.jpg",
                    ext: "jpg",
                    kind: "image",
                    size: 1,
                    mtime_ns: 1,
                    sync_state: "ok",
                }],
            )
            .await
            .is_err());
        sqlx::query("ROLLBACK").execute(&mut *locker).await.unwrap();
    }

    #[tokio::test]
    async fn count_for_root_includes_missing_assets() {
        let (catalog, _dir) = test_catalog().await;
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let assets = AssetRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root("/tmp/p", "local", "watch", None)
            .await
            .unwrap();
        let asset = assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "missing.jpg",
                file_name: "missing.jpg",
                ext: "jpg",
                kind: "image",
                size: 1,
                mtime_ns: 1,
                sync_state: "missing",
            })
            .await
            .unwrap();
        let (total, missing) = assets.count_for_root(root.id).await.unwrap();
        assert_eq!(total, 1);
        assert_eq!(missing, 1);
        assert_eq!(asset.sync_state, "missing");
    }

    #[tokio::test]
    async fn upsert_assets_batch_commits_multiple_rows() {
        let (catalog, _dir) = test_catalog().await;
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let assets = AssetRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root("/tmp/p", "local", "watch", None)
            .await
            .unwrap();
        let ids = assets
            .upsert_assets_batch(
                root.id,
                &[
                    UpsertAssetInput {
                        root_id: root.id,
                        rel_path: "one.jpg",
                        file_name: "one.jpg",
                        ext: "jpg",
                        kind: "image",
                        size: 1,
                        mtime_ns: 1,
                        sync_state: "new",
                    },
                    UpsertAssetInput {
                        root_id: root.id,
                        rel_path: "two.jpg",
                        file_name: "two.jpg",
                        ext: "jpg",
                        kind: "image",
                        size: 2,
                        mtime_ns: 2,
                        sync_state: "new",
                    },
                    UpsertAssetInput {
                        root_id: root.id,
                        rel_path: "three.jpg",
                        file_name: "three.jpg",
                        ext: "jpg",
                        kind: "image",
                        size: 3,
                        mtime_ns: 3,
                        sync_state: "new",
                    },
                ],
            )
            .await
            .unwrap();
        assert_eq!(ids.len(), 3);
        let (total, missing) = assets.count_for_root(root.id).await.unwrap();
        assert_eq!(total, 3);
        assert_eq!(missing, 0);
    }
}
