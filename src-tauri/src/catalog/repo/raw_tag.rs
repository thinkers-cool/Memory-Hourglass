use crate::error::Result;
use sqlx::SqlitePool;

pub struct RawTagRepo {
    pool: SqlitePool,
}

impl RawTagRepo {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn list_for_asset(
        &self,
        asset_id: i64,
    ) -> Result<Vec<crate::catalog::models::RawTag>> {
        Ok(sqlx::query_as::<_, crate::catalog::models::RawTag>(
            "SELECT name, value FROM asset_raw_tag WHERE asset_id = ? ORDER BY name, value",
        )
        .bind(asset_id)
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn count_for_asset(&self, asset_id: i64) -> Result<i64> {
        Ok(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM asset_raw_tag WHERE asset_id = ?")
                .bind(asset_id)
                .fetch_one(&self.pool)
                .await?,
        )
    }

    pub async fn replace_for_asset(
        &self,
        asset_id: i64,
        tags: &[crate::catalog::models::RawTag],
    ) -> Result<()> {
        sqlx::query("DELETE FROM asset_raw_tag WHERE asset_id = ?")
            .bind(asset_id)
            .execute(&self.pool)
            .await?;
        for tag in tags {
            sqlx::query("INSERT INTO asset_raw_tag (asset_id, name, value) VALUES (?, ?, ?)")
                .bind(asset_id)
                .bind(&tag.name)
                .bind(&tag.value)
                .execute(&self.pool)
                .await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::repo::test_support::test_catalog;
    use crate::catalog::repo::{AssetRepo, SourceRootRepo, UpsertAssetInput};

    #[tokio::test]
    async fn raw_tag_count_and_replace() {
        let (catalog, _dir) = test_catalog().await;
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let assets = AssetRepo::new(catalog.pool().clone());
        let raw_tag_repo = RawTagRepo::new(catalog.pool().clone());
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
        assert_eq!(raw_tag_repo.count_for_asset(asset.id).await.unwrap(), 0);
        raw_tag_repo
            .replace_for_asset(
                asset.id,
                &[crate::catalog::models::RawTag {
                    name: "Make".into(),
                    value: "Test".into(),
                }],
            )
            .await
            .unwrap();
        assert_eq!(raw_tag_repo.count_for_asset(asset.id).await.unwrap(), 1);
    }

    #[tokio::test]
    async fn raw_tag_replace_clears_tags() {
        let (catalog, _dir) = test_catalog().await;
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let assets = AssetRepo::new(catalog.pool().clone());
        let raw_tag_repo = RawTagRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root("/tmp/p", "local", "watch", None)
            .await
            .unwrap();
        let asset = assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "raw.jpg",
                file_name: "raw.jpg",
                ext: "jpg",
                kind: "image",
                size: 1,
                mtime_ns: 1,
                sync_state: "ok",
            })
            .await
            .unwrap();
        raw_tag_repo
            .replace_for_asset(
                asset.id,
                &[crate::catalog::models::RawTag {
                    name: "Make".into(),
                    value: "Test".into(),
                }],
            )
            .await
            .unwrap();
        raw_tag_repo.replace_for_asset(asset.id, &[]).await.unwrap();
        assert_eq!(raw_tag_repo.count_for_asset(asset.id).await.unwrap(), 0);
    }

    #[tokio::test]
    async fn raw_tag_list_for_asset_returns_replaced_tags() {
        let (catalog, _dir) = test_catalog().await;
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let assets = AssetRepo::new(catalog.pool().clone());
        let raw_tag_repo = RawTagRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root("/tmp/p", "local", "watch", None)
            .await
            .unwrap();
        let asset = assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "listed.jpg",
                file_name: "listed.jpg",
                ext: "jpg",
                kind: "image",
                size: 1,
                mtime_ns: 1,
                sync_state: "ok",
            })
            .await
            .unwrap();
        raw_tag_repo
            .replace_for_asset(
                asset.id,
                &[crate::catalog::models::RawTag {
                    name: "Model".into(),
                    value: "Alpha".into(),
                }],
            )
            .await
            .unwrap();
        let tags = raw_tag_repo.list_for_asset(asset.id).await.unwrap();
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].name, "Model");
    }

    #[tokio::test]
    async fn replace_raw_tags_inserts_multiple_values() {
        let (catalog, _dir) = test_catalog().await;
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let assets = AssetRepo::new(catalog.pool().clone());
        let raw_tag_repo = RawTagRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root("/tmp/p", "local", "watch", None)
            .await
            .unwrap();
        let asset = assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "raw.jpg",
                file_name: "raw.jpg",
                ext: "jpg",
                kind: "image",
                size: 1,
                mtime_ns: 1,
                sync_state: "ok",
            })
            .await
            .unwrap();
        raw_tag_repo
            .replace_for_asset(
                asset.id,
                &[
                    crate::catalog::models::RawTag {
                        name: "Make".into(),
                        value: "Sony".into(),
                    },
                    crate::catalog::models::RawTag {
                        name: "Model".into(),
                        value: "ILCE-7C".into(),
                    },
                ],
            )
            .await
            .unwrap();
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM asset_raw_tag WHERE asset_id = ?")
                .bind(asset.id)
                .fetch_one(catalog.pool())
                .await
                .unwrap();
        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn raw_tag_repo_errors_after_pool_close() {
        let (catalog, _dir) = test_catalog().await;
        let pool = catalog.pool().clone();
        let roots = SourceRootRepo::new(pool.clone());
        let assets = AssetRepo::new(pool.clone());
        let raw_tag_repo = RawTagRepo::new(pool.clone());
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
        assert!(raw_tag_repo.list_for_asset(asset.id).await.is_err());
        assert!(raw_tag_repo.count_for_asset(asset.id).await.is_err());
        assert!(raw_tag_repo
            .replace_for_asset(
                asset.id,
                &[crate::catalog::models::RawTag {
                    name: "Make".into(),
                    value: "Test".into(),
                }],
            )
            .await
            .is_err());
    }

    #[tokio::test]
    async fn list_scan_state_for_root_includes_raw_tag_count() {
        let (catalog, _dir) = test_catalog().await;
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let assets = AssetRepo::new(catalog.pool().clone());
        let raw_tag_repo = RawTagRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root("/tmp/p", "local", "watch", None)
            .await
            .unwrap();
        let asset = assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "scan.jpg",
                file_name: "scan.jpg",
                ext: "jpg",
                kind: "image",
                size: 1,
                mtime_ns: 1,
                sync_state: "ok",
            })
            .await
            .unwrap();
        assets.set_thumb_key(asset.id, "thumb.webp").await.unwrap();
        assets.mark_indexed(asset.id, 1).await.unwrap();
        raw_tag_repo
            .replace_for_asset(
                asset.id,
                &[crate::catalog::models::RawTag {
                    name: "Make".into(),
                    value: "Test".into(),
                }],
            )
            .await
            .unwrap();
        let states = assets.list_scan_state_for_root(root.id).await.unwrap();
        let state = states
            .iter()
            .find(|row| row.rel_path == "scan.jpg")
            .unwrap();
        assert_eq!(state.raw_tag_count, 1);
        assert_eq!(state.thumb_key.as_deref(), Some("thumb.webp"));
    }

    #[tokio::test]
    async fn raw_tag_replace_errors_under_exclusive_lock() {
        let (catalog, _dir) = test_catalog().await;
        let pool = catalog.pool().clone();
        let roots = SourceRootRepo::new(pool.clone());
        let assets = AssetRepo::new(pool.clone());
        let raw_tag_repo = RawTagRepo::new(pool.clone());
        let root = roots
            .insert_root("/tmp/p", "local", "watch", None)
            .await
            .unwrap();
        let asset = assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "raw.jpg",
                file_name: "raw.jpg",
                ext: "jpg",
                kind: "image",
                size: 1,
                mtime_ns: 1,
                sync_state: "ok",
            })
            .await
            .unwrap();
        let mut locker = pool.acquire().await.unwrap();
        sqlx::query("BEGIN EXCLUSIVE")
            .execute(&mut *locker)
            .await
            .unwrap();
        assert!(raw_tag_repo
            .replace_for_asset(
                asset.id,
                &[crate::catalog::models::RawTag {
                    name: "Make".into(),
                    value: "Test".into(),
                }],
            )
            .await
            .is_err());
        sqlx::query("ROLLBACK").execute(&mut *locker).await.unwrap();
    }
}
