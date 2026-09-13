use crate::catalog::models::AssetMeta;
use crate::catalog::pools::CatalogPools;
use crate::error::Result;

pub struct AssetMetaRepo {
    pools: CatalogPools,
}

impl AssetMetaRepo {
    pub fn new(pools: CatalogPools) -> Self {
        Self { pools }
    }

    pub async fn upsert(&self, meta: &AssetMeta) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO asset_meta (asset_id, capture_at, camera, lens, rating, latitude, longitude, keywords_json, rotation)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(asset_id) DO UPDATE SET
                capture_at = excluded.capture_at,
                camera = excluded.camera,
                lens = excluded.lens,
                rating = excluded.rating,
                latitude = excluded.latitude,
                longitude = excluded.longitude,
                keywords_json = excluded.keywords_json,
                rotation = excluded.rotation
            "#,
        )
        .bind(meta.asset_id)
        .bind(meta.capture_at)
        .bind(&meta.camera)
        .bind(&meta.lens)
        .bind(meta.rating)
        .bind(meta.latitude)
        .bind(meta.longitude)
        .bind(&meta.keywords_json)
        .bind(meta.rotation)
        .execute(self.pools.write())
        .await?;
        Ok(())
    }

    pub async fn get(&self, asset_id: i64) -> Result<Option<AssetMeta>> {
        Ok(
            sqlx::query_as::<_, AssetMeta>("SELECT * FROM asset_meta WHERE asset_id = ?")
                .bind(asset_id)
                .fetch_optional(self.pools.read())
                .await?,
        )
    }

    pub async fn get_batch(
        &self,
        asset_ids: &[i64],
    ) -> Result<std::collections::HashMap<i64, AssetMeta>> {
        if asset_ids.is_empty() {
            return Ok(std::collections::HashMap::new());
        }
        let mut builder = sqlx::QueryBuilder::new(
            "SELECT asset_id, capture_at, camera, lens, rating, latitude, longitude, keywords_json, rotation FROM asset_meta WHERE asset_id IN (",
        );
        let mut separated = builder.separated(", ");
        for asset_id in asset_ids {
            separated.push_bind(asset_id);
        }
        separated.push_unseparated(")");
        let rows = builder
            .build_query_as::<AssetMeta>()
            .fetch_all(self.pools.read())
            .await?;
        Ok(rows.into_iter().map(|meta| (meta.asset_id, meta)).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::models::AssetMeta;
    use crate::catalog::repo::test_support::test_catalog;
    use crate::catalog::repo::{AssetRepo, SourceRootRepo, TagRepo, UpsertAssetInput};

    #[tokio::test]
    async fn meta_and_tags_roundtrip() {
        let (catalog, _dir) = test_catalog().await;
        let roots = SourceRootRepo::new(catalog.pools().clone());
        let assets = AssetRepo::new(catalog.pools().clone());
        let meta_repo = AssetMetaRepo::new(catalog.pools().clone());
        let tag_repo = TagRepo::new(catalog.pools().clone());

        let root = roots
            .insert_root("/tmp/p", "local", "watch", None)
            .await
            .unwrap();
        let asset = assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "c.jpg",
                file_name: "c.jpg",
                ext: "jpg",
                kind: "image",
                size: 1,
                mtime_ns: 1,
                sync_state: "ok",
            })
            .await
            .unwrap();

        meta_repo
            .upsert(&AssetMeta {
                asset_id: asset.id,
                capture_at: Some(1_700_000_000),
                camera: Some("ILCE-7C".into()),
                lens: None,
                rating: Some(4),
                latitude: None,
                longitude: None,
                keywords_json: Some(r#"["travel"]"#.into()),
                rotation: None,
            })
            .await
            .unwrap();

        let travel_id = tag_repo.create_tag("travel", None, None).await.unwrap();
        let japan_id = tag_repo.create_tag("japan", None, None).await.unwrap();
        tag_repo
            .append_tag_id_to_assets(&[asset.id], travel_id)
            .await
            .unwrap();
        tag_repo
            .append_tag_id_to_assets(&[asset.id], japan_id)
            .await
            .unwrap();

        let meta = meta_repo.get(asset.id).await.unwrap().unwrap();
        assert_eq!(meta.rating, Some(4));
        let tag_ids = tag_repo.list_ids_for_asset(asset.id).await.unwrap();
        assert_eq!(tag_ids.len(), 2);
    }

    #[tokio::test]
    async fn asset_meta_get_and_upsert_update() {
        let (catalog, _dir) = test_catalog().await;
        let roots = SourceRootRepo::new(catalog.pools().clone());
        let assets = AssetRepo::new(catalog.pools().clone());
        let meta_repo = AssetMetaRepo::new(catalog.pools().clone());
        let root = roots
            .insert_root("/tmp/p", "local", "watch", None)
            .await
            .unwrap();
        let asset = assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "meta.jpg",
                file_name: "meta.jpg",
                ext: "jpg",
                kind: "image",
                size: 1,
                mtime_ns: 1,
                sync_state: "ok",
            })
            .await
            .unwrap();
        assert!(meta_repo.get(asset.id).await.unwrap().is_none());
        meta_repo
            .upsert(&AssetMeta {
                asset_id: asset.id,
                capture_at: None,
                camera: None,
                lens: None,
                rating: Some(3),
                latitude: None,
                longitude: None,
                keywords_json: None,
                rotation: None,
            })
            .await
            .unwrap();
        meta_repo
            .upsert(&AssetMeta {
                asset_id: asset.id,
                capture_at: None,
                camera: None,
                lens: None,
                rating: Some(5),
                latitude: None,
                longitude: None,
                keywords_json: None,
                rotation: None,
            })
            .await
            .unwrap();
        assert_eq!(
            meta_repo.get(asset.id).await.unwrap().unwrap().rating,
            Some(5)
        );
    }

    #[tokio::test]
    async fn meta_repo_errors_after_pool_close() {
        let (catalog, _dir) = test_catalog().await;
        let pools = catalog.pools().clone();
        let roots = SourceRootRepo::new(pools.clone());
        let assets = AssetRepo::new(pools.clone());
        let meta_repo = AssetMetaRepo::new(pools.clone());
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
        pools.close().await;
        assert!(meta_repo
            .upsert(&AssetMeta {
                asset_id: asset.id,
                capture_at: None,
                camera: None,
                lens: None,
                rating: Some(1),
                latitude: None,
                longitude: None,
                keywords_json: None,
                rotation: None,
            })
            .await
            .is_err());
        assert!(meta_repo.get(asset.id).await.is_err());
    }
}
