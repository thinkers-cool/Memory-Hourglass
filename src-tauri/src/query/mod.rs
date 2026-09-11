use crate::catalog::models::{AssetCard, AssetDetail, AssetMetaPatch};
use crate::catalog::repo::{AssetMetaRepo, AssetRepo, RawTagRepo, SourceRootRepo, TagRepo};
use crate::dates::CAPTURE_AT_SQL;
use crate::error::{AppError, Result};
use crate::link::LinkService;
use crate::metadata::{metadata_context_for_asset, MetadataContext, MetadataService};
use crate::query::display_path::resolve_display_path;
use crate::scan::index_asset::index_asset_on_disk;
use crate::scan::index_integrity::is_index_complete;
use crate::scan::index_pipeline::{apply_index_output, IndexApplyInput};
use crate::sort::SortSpec;
use crate::workspace::WorkspaceMediaSettings;
use serde::{Deserialize, Serialize};
use sqlx::QueryBuilder;
use sqlx::SqlitePool;
use std::path::PathBuf;

mod display_path;
mod filter;
mod metadata_refresh;
pub(crate) mod tag_keywords;

pub use filter::AssetFilter;

use filter::{apply_deleted_clause, apply_filter, apply_sort, Row};
use metadata_refresh::refresh_asset_after_metadata_write;
use tag_keywords::sync_assets_tag_keywords;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub total: i64,
    pub items: Vec<AssetCard>,
}

pub struct QueryService {
    pool: SqlitePool,
    thumb_dir: PathBuf,
    media_settings: WorkspaceMediaSettings,
}

impl QueryService {
    pub fn new(pool: SqlitePool, thumb_dir: PathBuf) -> Self {
        Self::with_media_settings(pool, thumb_dir, WorkspaceMediaSettings::default())
    }

    pub fn with_media_settings(
        pool: SqlitePool,
        thumb_dir: PathBuf,
        media_settings: WorkspaceMediaSettings,
    ) -> Self {
        Self {
            pool,
            thumb_dir,
            media_settings,
        }
    }

    fn metadata_ctx(&self, root_id: i64, rel_path: &str, abs_path: PathBuf) -> MetadataContext {
        metadata_context_for_asset(
            self.media_settings.read_only,
            &self.media_settings.workspace_xmp_dir,
            root_id,
            rel_path,
            abs_path,
        )
    }

    async fn resolve_filter(&self, filter: &AssetFilter) -> Result<AssetFilter> {
        let mut resolved = filter.clone();
        if let Some(tag_ids) = &filter.tag_ids {
            if !tag_ids.is_empty() {
                let tag_repo = TagRepo::new(self.pool.clone());
                resolved.tag_ids = Some(tag_repo.expand_tag_ids_with_descendants(tag_ids).await?);
            }
        }
        Ok(resolved)
    }

    async fn count_resolved(&self, filter: &AssetFilter) -> Result<i64> {
        let mut builder = QueryBuilder::new(
            "SELECT COUNT(*) FROM asset a LEFT JOIN asset_meta m ON m.asset_id = a.id JOIN source_root r ON r.id = a.root_id WHERE ",
        );
        apply_deleted_clause(&mut builder, filter);
        apply_filter(&mut builder, filter);
        let query = builder.build_query_scalar::<i64>();
        Ok(query.fetch_one(&self.pool).await?)
    }

    pub async fn count(&self, filter: &AssetFilter) -> Result<i64> {
        let filter = self.resolve_filter(filter).await?;
        self.count_resolved(&filter).await
    }

    pub async fn query(
        &self,
        filter: &AssetFilter,
        sort: &str,
        offset: i64,
        limit: i64,
    ) -> Result<QueryResult> {
        let filter = self.resolve_filter(filter).await?;
        let total = self.count_resolved(&filter).await?;
        let sort_spec = SortSpec::parse(sort)?;
        let (sort_field, sort_desc) = sort_spec.query_order_sql();

        let mut builder = QueryBuilder::new(format!(
            "SELECT a.id, a.file_name, a.ext, a.kind, {} as capture_at, m.rating, a.sync_state, a.thumb_key, a.has_duplicate, r.path as root_path, a.rel_path FROM asset a LEFT JOIN asset_meta m ON m.asset_id = a.id JOIN source_root r ON r.id = a.root_id WHERE ",
            CAPTURE_AT_SQL
        ));
        apply_deleted_clause(&mut builder, &filter);
        apply_filter(&mut builder, &filter);
        apply_sort(&mut builder, sort_field, sort_desc);
        builder.push(" LIMIT ").push_bind(limit);
        builder.push(" OFFSET ").push_bind(offset);

        let rows = builder
            .build_query_as::<Row>()
            .fetch_all(&self.pool)
            .await?;

        let items = rows
            .into_iter()
            .map(|r| {
                let abs = PathBuf::from(&r.root_path).join(&r.rel_path);
                let thumb_path = r
                    .thumb_key
                    .as_ref()
                    .map(|k| self.thumb_dir.join(k).to_string_lossy().to_string());
                AssetCard {
                    id: r.id,
                    file_name: r.file_name,
                    ext: r.ext,
                    kind: r.kind,
                    capture_at: r.capture_at,
                    rating: r.rating,
                    sync_state: r.sync_state,
                    thumb_path,
                    abs_path: abs.to_string_lossy().to_string(),
                    has_duplicate: r.has_duplicate != 0,
                }
            })
            .collect();

        Ok(QueryResult { total, items })
    }

    pub async fn get_detail(&self, asset_id: i64) -> Result<AssetDetail> {
        let assets = AssetRepo::new(self.pool.clone());
        let meta_repo = AssetMetaRepo::new(self.pool.clone());
        let tag_repo = TagRepo::new(self.pool.clone());
        let raw_tag_repo = RawTagRepo::new(self.pool.clone());
        let roots = SourceRootRepo::new(self.pool.clone());

        let asset = assets.get_asset(asset_id).await?;
        let root = roots.get_root(asset.root_id).await?;
        let abs_path = PathBuf::from(&root.path).join(&asset.rel_path);

        let needs_repair = !is_index_complete(
            asset.mtime_ns,
            asset.indexed_mtime_ns,
            asset.thumb_key.as_deref(),
            &asset.kind,
        );
        let (meta, raw_tags) = if needs_repair {
            let prior_thumb_key = asset.thumb_key.clone();
            let abs_path_for_index = abs_path.clone();
            let thumb_dir = self.thumb_dir.clone();
            let metadata_ctx =
                self.metadata_ctx(asset.root_id, &asset.rel_path, abs_path_for_index.clone());
            let indexed = tokio::task::spawn_blocking(move || {
                index_asset_on_disk(asset_id, &abs_path_for_index, &thumb_dir, &metadata_ctx)
            })
            .await
            .map_err(|e| AppError::Catalog(e.to_string()))?;

            apply_index_output(
                &self.pool,
                &IndexApplyInput {
                    asset_id,
                    mtime_ns: asset.mtime_ns,
                    kind: asset.kind.clone(),
                    prior_thumb_key,
                    indexed,
                },
            )
            .await?;

            (
                meta_repo.get(asset_id).await?,
                raw_tag_repo.list_for_asset(asset_id).await?,
            )
        } else {
            (
                meta_repo.get(asset_id).await?,
                raw_tag_repo.list_for_asset(asset_id).await?,
            )
        };

        let tag_ids = tag_repo.list_ids_for_asset(asset_id).await?;
        let album_ids = crate::collection::CollectionRepo::new(self.pool.clone())
            .list_album_ids_for_asset(asset_id)
            .await?;
        let link_service = LinkService::new(self.pool.clone());
        let links = link_service.list_links(asset_id).await?;
        let duplicates = link_service.list_duplicates_for_asset(asset_id).await?;
        let abs_path_str = abs_path.to_string_lossy().to_string();
        let display_path = resolve_display_path(&abs_path_str, &asset.kind, &links);

        Ok(AssetDetail {
            asset,
            meta,
            abs_path: abs_path_str,
            display_path,
            tag_ids,
            album_ids,
            raw_tags,
            links,
            duplicates,
        })
    }

    pub async fn apply_meta_patch(
        &self,
        asset_id: i64,
        patch: AssetMetaPatch,
    ) -> Result<AssetDetail> {
        self.apply_meta_patch_inner(asset_id, patch).await?;
        LinkService::new(self.pool.clone())
            .refresh_duplicate_index()
            .await?;
        self.get_detail(asset_id).await
    }

    pub async fn batch_apply_meta(&self, asset_ids: &[i64], patch: AssetMetaPatch) -> Result<u64> {
        for asset_id in asset_ids {
            self.apply_meta_patch_inner(*asset_id, patch.clone())
                .await?;
        }
        if !asset_ids.is_empty() {
            refresh_duplicate_index(&self.pool).await?;
        }
        Ok(asset_ids.len() as u64)
    }

    async fn apply_meta_patch_inner(&self, asset_id: i64, patch: AssetMetaPatch) -> Result<()> {
        let assets = AssetRepo::new(self.pool.clone());
        let roots = SourceRootRepo::new(self.pool.clone());

        let asset = assets.get_asset(asset_id).await?;
        let root = roots.get_root(asset.root_id).await?;
        let abs_path = PathBuf::from(&root.path).join(&asset.rel_path);

        let metadata_ctx = self.metadata_ctx(asset.root_id, &asset.rel_path, abs_path.clone());
        if let Some(rating) = patch.rating {
            MetadataService::write_rating(&metadata_ctx, rating)?;
        }
        refresh_asset_after_metadata_write(
            &self.pool,
            asset_id,
            &abs_path,
            &metadata_ctx,
            self.media_settings.read_only,
        )
        .await?;
        Ok(())
    }

    pub async fn batch_append_tags(&self, asset_ids: &[i64], tag_id: i64) -> Result<u64> {
        let tag_repo = TagRepo::new(self.pool.clone());
        let updated = tag_repo.append_tag_id_to_assets(asset_ids, tag_id).await?;
        sync_assets_tag_keywords(&self.pool, asset_ids, &self.media_settings).await?;
        Ok(updated)
    }

    pub async fn batch_remove_tags(&self, asset_ids: &[i64], tag_id: i64) -> Result<u64> {
        let tag_repo = TagRepo::new(self.pool.clone());
        let updated = tag_repo
            .remove_tag_id_from_assets(asset_ids, tag_id)
            .await?;
        sync_assets_tag_keywords(&self.pool, asset_ids, &self.media_settings).await?;
        Ok(updated)
    }
}

async fn refresh_duplicate_index(pool: &sqlx::SqlitePool) -> Result<()> {
    LinkService::new(pool.clone())
        .refresh_duplicate_index()
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::models::AssetMeta;
    use crate::catalog::repo::{AssetMetaRepo, AssetRepo, SourceRootRepo, TagRepo};
    use crate::catalog::Catalog;
    use crate::scan::{ScanControl, ScanService};
    use tempfile::tempdir;

    #[tokio::test]
    async fn query_filters_by_rating() {
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        std::fs::write(
            photos.join("high.jpg"),
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();

        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let thumb_dir = dir.path().join("thumbs");
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();

        ScanService::new(catalog.pool().clone(), thumb_dir.clone())
            .scan_root(root.id, &ScanControl::noop())
            .await
            .unwrap();

        let assets = AssetRepo::new(catalog.pool().clone());
        let asset = assets
            .find_by_path(root.id, "high.jpg")
            .await
            .unwrap()
            .unwrap();
        AssetMetaRepo::new(catalog.pool().clone())
            .upsert(&AssetMeta {
                asset_id: asset.id,
                capture_at: Some(1_700_000_000),
                camera: Some("ILCE-7C".into()),
                lens: None,
                rating: Some(5),
                latitude: None,
                longitude: None,
                keywords_json: None,
            })
            .await
            .unwrap();

        let query = QueryService::new(catalog.pool().clone(), thumb_dir);
        let result = query
            .query(
                &AssetFilter {
                    rating_min: Some(4),
                    ..Default::default()
                },
                "date:desc",
                0,
                50,
            )
            .await
            .unwrap();

        assert_eq!(result.total, 1);
        assert_eq!(result.items[0].rating, Some(5));
    }

    #[tokio::test]
    async fn query_date_filter_excludes_assets_without_embedded_capture_date() {
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        std::fs::write(photos.join("clip.mp4"), b"not-a-real-video").unwrap();
        std::fs::write(
            photos.join("dated.jpg"),
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();

        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let thumb_dir = dir.path().join("thumbs");
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();

        ScanService::new(catalog.pool().clone(), thumb_dir.clone())
            .scan_root(root.id, &ScanControl::noop())
            .await
            .unwrap();

        let assets = AssetRepo::new(catalog.pool().clone());
        let clip = assets
            .find_by_path(root.id, "clip.mp4")
            .await
            .unwrap()
            .unwrap();
        let dated = assets
            .find_by_path(root.id, "dated.jpg")
            .await
            .unwrap()
            .unwrap();
        let capture_at = 1_710_000_000i64;
        sqlx::query(
            "INSERT INTO asset_meta (asset_id, capture_at) VALUES (?, ?) ON CONFLICT(asset_id) DO UPDATE SET capture_at = excluded.capture_at",
        )
        .bind(dated.id)
        .bind(capture_at)
        .execute(catalog.pool())
        .await
        .unwrap();

        let query = QueryService::new(catalog.pool().clone(), thumb_dir);
        let result = query
            .query(
                &AssetFilter {
                    capture_from: Some(capture_at - 60),
                    capture_to: Some(capture_at + 60),
                    ..Default::default()
                },
                "date:desc",
                0,
                50,
            )
            .await
            .unwrap();

        assert_eq!(result.total, 1);
        assert_eq!(result.items[0].id, dated.id);
        assert!(result.items.iter().all(|item| item.id != clip.id));
    }

    #[tokio::test]
    async fn query_filters_by_album() {
        use crate::collection::CollectionRepo;

        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        std::fs::write(
            photos.join("one.jpg"),
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();
        std::fs::write(
            photos.join("two.jpg"),
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();

        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let thumb_dir = dir.path().join("thumbs");
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();

        ScanService::new(catalog.pool().clone(), thumb_dir.clone())
            .scan_root(root.id, &ScanControl::noop())
            .await
            .unwrap();

        let assets = AssetRepo::new(catalog.pool().clone());
        let one = assets
            .find_by_path(root.id, "one.jpg")
            .await
            .unwrap()
            .unwrap();
        let two = assets
            .find_by_path(root.id, "two.jpg")
            .await
            .unwrap()
            .unwrap();

        let collection = CollectionRepo::new(catalog.pool().clone());
        let album = collection
            .create_album("Trip", "date:desc", None)
            .await
            .unwrap();
        collection
            .set_album_items(album.id, &[one.id])
            .await
            .unwrap();

        let query = QueryService::new(catalog.pool().clone(), thumb_dir);
        let filtered = query
            .query(
                &AssetFilter {
                    album_ids: Some(vec![album.id]),
                    ..Default::default()
                },
                "date:desc",
                0,
                50,
            )
            .await
            .unwrap();

        assert_eq!(filtered.total, 1);
        assert_eq!(filtered.items[0].id, one.id);
        assert!(filtered.items.iter().all(|item| item.id != two.id));
    }

    #[tokio::test]
    async fn query_filters_by_parent_tag_include_descendants() {
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        std::fs::write(
            photos.join("child.jpg"),
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();

        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let thumb_dir = dir.path().join("thumbs");
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();

        ScanService::new(catalog.pool().clone(), thumb_dir.clone())
            .scan_root(root.id, &ScanControl::noop())
            .await
            .unwrap();

        let assets = AssetRepo::new(catalog.pool().clone());
        let asset = assets
            .find_by_path(root.id, "child.jpg")
            .await
            .unwrap()
            .unwrap();
        let tag_repo = TagRepo::new(catalog.pool().clone());
        let parent_id = tag_repo.create_tag("travel", None, None).await.unwrap();
        let child_id = tag_repo
            .create_tag("japan", Some(parent_id), None)
            .await
            .unwrap();
        tag_repo
            .append_tag_id_to_assets(&[asset.id], child_id)
            .await
            .unwrap();

        let query = QueryService::new(catalog.pool().clone(), thumb_dir);
        let filtered = query
            .query(
                &AssetFilter {
                    tag_ids: Some(vec![parent_id]),
                    ..Default::default()
                },
                "date:desc",
                0,
                50,
            )
            .await
            .unwrap();

        assert_eq!(filtered.total, 1);
        assert_eq!(filtered.items[0].id, asset.id);
    }

    #[tokio::test]
    async fn query_applies_extended_filters_and_sorts() {
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        std::fs::write(
            photos.join("alpha.jpg"),
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();
        std::fs::write(
            photos.join("beta.jpg"),
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();

        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let thumb_dir = dir.path().join("thumbs");
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        ScanService::new(catalog.pool().clone(), thumb_dir.clone())
            .scan_root(root.id, &ScanControl::noop())
            .await
            .unwrap();

        let assets = AssetRepo::new(catalog.pool().clone());
        let alpha = assets
            .find_by_path(root.id, "alpha.jpg")
            .await
            .unwrap()
            .unwrap();
        let beta = assets
            .find_by_path(root.id, "beta.jpg")
            .await
            .unwrap()
            .unwrap();
        AssetMetaRepo::new(catalog.pool().clone())
            .upsert(&AssetMeta {
                asset_id: alpha.id,
                capture_at: Some(1_700_000_000),
                camera: Some("Canon EOS".into()),
                lens: Some("50mm".into()),
                rating: Some(5),
                latitude: Some(35.0),
                longitude: Some(139.0),
                keywords_json: None,
            })
            .await
            .unwrap();
        AssetMetaRepo::new(catalog.pool().clone())
            .upsert(&AssetMeta {
                asset_id: beta.id,
                capture_at: Some(1_600_000_000),
                camera: Some("Sony".into()),
                lens: None,
                rating: Some(2),
                latitude: None,
                longitude: None,
                keywords_json: None,
            })
            .await
            .unwrap();

        let query = QueryService::new(catalog.pool().clone(), thumb_dir);
        let filtered_count = query
            .count(&AssetFilter {
                root_id: Some(root.id),
                kind: Some("image".into()),
                rating_min: Some(4),
                has_gps: Some(true),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(filtered_count, 1);

        for sort in [
            "name:asc",
            "name:desc",
            "rating:asc",
            "path:asc",
            "date:asc",
        ] {
            let page = query
                .query(&AssetFilter::default(), sort, 0, 10)
                .await
                .unwrap();
            assert_eq!(page.total, 2);
        }

        let search = query
            .query(
                &AssetFilter {
                    meta_search: Some("alpha".into()),
                    ..Default::default()
                },
                "date:desc",
                0,
                10,
            )
            .await
            .unwrap();
        assert_eq!(search.total, 1);

        let empty_ids = query
            .query(
                &AssetFilter {
                    asset_ids: Some(vec![]),
                    ..Default::default()
                },
                "date:desc",
                0,
                10,
            )
            .await
            .unwrap();
        assert_eq!(empty_ids.total, 0);

        assets.soft_delete(&[beta.id], 1).await.unwrap();
        let deleted = query
            .query(
                &AssetFilter {
                    deleted_only: Some(true),
                    ..Default::default()
                },
                "date:desc",
                0,
                10,
            )
            .await
            .unwrap();
        assert_eq!(deleted.total, 1);
    }

    #[tokio::test]
    async fn get_detail_returns_not_found_for_missing_asset() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let query = QueryService::new(catalog.pool().clone(), dir.path().join("thumbs"));
        let err = query.get_detail(999_999).await.unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[tokio::test]
    async fn get_detail_repairs_incomplete_index() {
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        std::fs::write(
            photos.join("repair.jpg"),
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();

        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let thumb_dir = dir.path().join("thumbs");
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        let assets = AssetRepo::new(catalog.pool().clone());
        let asset = assets
            .upsert_asset(crate::catalog::repo::UpsertAssetInput {
                root_id: root.id,
                rel_path: "repair.jpg".into(),
                file_name: "repair.jpg".into(),
                ext: "jpg".into(),
                kind: "image".into(),
                size: 1,
                mtime_ns: 99,
                sync_state: "ok",
            })
            .await
            .unwrap();

        let query = QueryService::new(catalog.pool().clone(), thumb_dir);
        let detail = query.get_detail(asset.id).await.unwrap();
        assert_eq!(detail.asset.id, asset.id);
        assert!(detail.meta.is_some());
    }

    #[tokio::test]
    async fn batch_tag_operations_return_updated_counts() {
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        std::fs::write(
            photos.join("tags.jpg"),
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let thumb_dir = dir.path().join("thumbs");
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        ScanService::new(catalog.pool().clone(), thumb_dir.clone())
            .scan_root(root.id, &ScanControl::noop())
            .await
            .unwrap();
        let asset = AssetRepo::new(catalog.pool().clone())
            .find_by_path(root.id, "tags.jpg")
            .await
            .unwrap()
            .unwrap();
        let tag_id = TagRepo::new(catalog.pool().clone())
            .create_tag("counted", None, None)
            .await
            .unwrap();
        let query = QueryService::new(catalog.pool().clone(), thumb_dir);
        let appended = query.batch_append_tags(&[asset.id], tag_id).await.unwrap();
        assert_eq!(appended, 1);
        let removed = query.batch_remove_tags(&[asset.id], tag_id).await.unwrap();
        assert_eq!(removed, 1);
    }

    #[tokio::test]
    async fn batch_meta_and_tags_update_keywords() {
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        std::fs::write(
            photos.join("tagged.jpg"),
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();

        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let thumb_dir = dir.path().join("thumbs");
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        ScanService::new(catalog.pool().clone(), thumb_dir.clone())
            .scan_root(root.id, &ScanControl::noop())
            .await
            .unwrap();
        let assets = AssetRepo::new(catalog.pool().clone());
        let asset = assets
            .find_by_path(root.id, "tagged.jpg")
            .await
            .unwrap()
            .unwrap();
        let tag_repo = TagRepo::new(catalog.pool().clone());
        let tag_id = tag_repo.create_tag("nature", None, None).await.unwrap();

        let query = QueryService::new(catalog.pool().clone(), thumb_dir);
        let count = query
            .batch_apply_meta(&[asset.id], AssetMetaPatch { rating: Some(3) })
            .await
            .unwrap();
        assert_eq!(count, 1);
        query.batch_append_tags(&[asset.id], tag_id).await.unwrap();
        query.batch_remove_tags(&[asset.id], tag_id).await.unwrap();
    }

    #[tokio::test]
    async fn query_filters_by_camera_duplicate_and_sync_state() {
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        std::fs::write(
            photos.join("dup.jpg"),
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();

        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let thumb_dir = dir.path().join("thumbs");
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        ScanService::new(catalog.pool().clone(), thumb_dir.clone())
            .scan_root(root.id, &ScanControl::noop())
            .await
            .unwrap();
        let assets = AssetRepo::new(catalog.pool().clone());
        let asset = assets
            .find_by_path(root.id, "dup.jpg")
            .await
            .unwrap()
            .unwrap();
        AssetMetaRepo::new(catalog.pool().clone())
            .upsert(&AssetMeta {
                asset_id: asset.id,
                capture_at: Some(1_700_000_000),
                camera: Some("Canon EOS R5".into()),
                lens: None,
                rating: Some(4),
                latitude: None,
                longitude: None,
                keywords_json: None,
            })
            .await
            .unwrap();
        sqlx::query("UPDATE asset SET has_duplicate = 1, sync_state = ? WHERE id = ?")
            .bind("new")
            .bind(asset.id)
            .execute(catalog.pool())
            .await
            .unwrap();

        let query = QueryService::new(catalog.pool().clone(), thumb_dir);
        let camera = query
            .count(&AssetFilter {
                camera: Some("Canon".into()),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(camera, 1);
        let dup = query
            .count(&AssetFilter {
                has_duplicate: Some(true),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(dup, 1);
        let sync = query
            .count(&AssetFilter {
                sync_states: Some(vec!["new".into()]),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(sync, 1);
        let by_id = query
            .query(
                &AssetFilter {
                    asset_ids: Some(vec![asset.id]),
                    ..Default::default()
                },
                "date:desc",
                0,
                10,
            )
            .await
            .unwrap();
        assert_eq!(by_id.total, 1);
        let empty_tags = query
            .query(
                &AssetFilter {
                    tag_ids: Some(vec![]),
                    ..Default::default()
                },
                "date:desc",
                0,
                10,
            )
            .await
            .unwrap();
        assert_eq!(empty_tags.total, 1);
    }

    #[tokio::test]
    async fn apply_meta_patch_refreshes_duplicate_index() {
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        std::fs::write(
            photos.join("meta.jpg"),
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();

        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let thumb_dir = dir.path().join("thumbs");
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        ScanService::new(catalog.pool().clone(), thumb_dir.clone())
            .scan_root(root.id, &ScanControl::noop())
            .await
            .unwrap();
        let assets = AssetRepo::new(catalog.pool().clone());
        let asset = assets
            .find_by_path(root.id, "meta.jpg")
            .await
            .unwrap()
            .unwrap();

        let query = QueryService::new(catalog.pool().clone(), thumb_dir);
        query
            .apply_meta_patch(asset.id, AssetMetaPatch { rating: Some(2) })
            .await
            .unwrap();
        let detail = query.get_detail(asset.id).await.unwrap();
        assert_eq!(detail.meta.as_ref().and_then(|m| m.rating), Some(2));
    }

    #[tokio::test]
    async fn batch_apply_meta_refreshes_duplicate_index() {
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        std::fs::write(
            photos.join("batch.jpg"),
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let thumb_dir = dir.path().join("thumbs");
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        ScanService::new(catalog.pool().clone(), thumb_dir.clone())
            .scan_root(root.id, &ScanControl::noop())
            .await
            .unwrap();
        let asset = AssetRepo::new(catalog.pool().clone())
            .find_by_path(root.id, "batch.jpg")
            .await
            .unwrap()
            .unwrap();
        let query = QueryService::new(catalog.pool().clone(), thumb_dir);
        let count = query
            .batch_apply_meta(&[asset.id], AssetMetaPatch { rating: Some(4) })
            .await
            .unwrap();
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn query_applies_multi_value_filters() {
        use crate::collection::CollectionRepo;

        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        for name in ["alpha.jpg", "beta.jpg"] {
            std::fs::write(
                photos.join(name),
                include_bytes!("../../tests/fixtures/minimal.jpg"),
            )
            .unwrap();
        }
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let thumb_dir = dir.path().join("thumbs");
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        ScanService::new(catalog.pool().clone(), thumb_dir.clone())
            .scan_root(root.id, &ScanControl::noop())
            .await
            .unwrap();
        let assets = AssetRepo::new(catalog.pool().clone());
        let alpha = assets
            .find_by_path(root.id, "alpha.jpg")
            .await
            .unwrap()
            .unwrap();
        let beta = assets
            .find_by_path(root.id, "beta.jpg")
            .await
            .unwrap()
            .unwrap();
        let collection = CollectionRepo::new(catalog.pool().clone());
        let album_a = collection
            .create_album("A", "date:desc", None)
            .await
            .unwrap();
        let album_b = collection
            .create_album("B", "date:desc", None)
            .await
            .unwrap();
        collection
            .set_album_items(album_a.id, &[alpha.id])
            .await
            .unwrap();
        collection
            .set_album_items(album_b.id, &[beta.id])
            .await
            .unwrap();

        let query = QueryService::new(catalog.pool().clone(), thumb_dir);
        let filtered = query
            .count(&AssetFilter {
                kind: Some("image".into()),
                sync_states: Some(vec!["ok".into(), "new".into()]),
                album_ids: Some(vec![album_a.id, album_b.id]),
                asset_ids: Some(vec![alpha.id, beta.id]),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(filtered, 2);
    }

    #[tokio::test]
    async fn query_pagination_honors_offset_and_limit() {
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        for name in ["p1.jpg", "p2.jpg", "p3.jpg"] {
            std::fs::write(
                photos.join(name),
                include_bytes!("../../tests/fixtures/minimal.jpg"),
            )
            .unwrap();
        }
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let thumb_dir = dir.path().join("thumbs");
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        ScanService::new(catalog.pool().clone(), thumb_dir.clone())
            .scan_root(root.id, &ScanControl::noop())
            .await
            .unwrap();
        let query = QueryService::new(catalog.pool().clone(), thumb_dir);
        let page1 = query
            .query(&AssetFilter::default(), "date:desc", 0, 1)
            .await
            .unwrap();
        let page2 = query
            .query(&AssetFilter::default(), "date:desc", 1, 1)
            .await
            .unwrap();
        assert_eq!(page1.total, 3);
        assert_eq!(page1.items.len(), 1);
        assert_eq!(page2.items.len(), 1);
        assert_ne!(page1.items[0].id, page2.items[0].id);
    }

    #[tokio::test]
    async fn query_rejects_invalid_sort_string() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let query = QueryService::new(catalog.pool().clone(), dir.path().join("thumbs"));
        assert!(query
            .query(&AssetFilter::default(), "bad-sort", 0, 10)
            .await
            .is_err());
    }

    #[tokio::test]
    async fn batch_apply_meta_empty_returns_zero() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let query = QueryService::new(catalog.pool().clone(), dir.path().join("thumbs"));
        let count = query
            .batch_apply_meta(&[], AssetMetaPatch { rating: Some(1) })
            .await
            .unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn get_detail_populates_links_albums_and_duplicates() {
        use crate::collection::CollectionRepo;
        use crate::link::LinkService;

        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        let jpeg = include_bytes!("../../tests/fixtures/minimal.jpg");
        std::fs::write(photos.join("DSC.arw"), jpeg).unwrap();
        std::fs::write(photos.join("DSC.jpg"), jpeg).unwrap();
        std::fs::write(photos.join("dup.jpg"), jpeg).unwrap();

        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let pool = catalog.pool().clone();
        let thumb_dir = dir.path().join("thumbs");
        let roots = SourceRootRepo::new(pool.clone());
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        ScanService::new(pool.clone(), thumb_dir.clone())
            .scan_root(root.id, &ScanControl::noop())
            .await
            .unwrap();

        let link = LinkService::new(pool.clone());
        link.link_raw_jpeg_in_root(root.id).await.unwrap();
        link.compute_hashes_for_root(root.id, &photos)
            .await
            .unwrap();
        link.refresh_duplicate_index().await.unwrap();

        let raw_id: i64 =
            sqlx::query_scalar("SELECT id FROM asset WHERE root_id = ? AND kind = 'raw' LIMIT 1")
                .bind(root.id)
                .fetch_one(&pool)
                .await
                .unwrap();

        let collection = CollectionRepo::new(pool.clone());
        let album = collection
            .create_album("Trip", "date:desc", None)
            .await
            .unwrap();
        collection
            .set_album_items(album.id, &[raw_id])
            .await
            .unwrap();

        let query = QueryService::new(pool.clone(), thumb_dir);
        let detail = query.get_detail(raw_id).await.unwrap();
        assert!(!detail.links.is_empty());
        assert_ne!(detail.display_path, detail.abs_path);
        assert_eq!(detail.album_ids, vec![album.id]);
        assert!(!detail.duplicates.is_empty());
    }

    #[tokio::test]
    async fn query_skips_empty_optional_filter_branches() {
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        std::fs::write(
            photos.join("solo.jpg"),
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();

        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let thumb_dir = dir.path().join("thumbs");
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        ScanService::new(catalog.pool().clone(), thumb_dir.clone())
            .scan_root(root.id, &ScanControl::noop())
            .await
            .unwrap();

        let query = QueryService::new(catalog.pool().clone(), thumb_dir);
        let filters = [
            AssetFilter {
                sync_states: Some(vec![]),
                ..Default::default()
            },
            AssetFilter {
                album_ids: Some(vec![]),
                ..Default::default()
            },
            AssetFilter {
                meta_search: Some("   ".into()),
                ..Default::default()
            },
        ];
        for filter in filters {
            assert_eq!(query.count(&filter).await.unwrap(), 1);
        }
    }

    #[tokio::test]
    async fn query_sort_table_covers_desc_branches() {
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        for name in ["first.jpg", "second.jpg"] {
            std::fs::write(
                photos.join(name),
                include_bytes!("../../tests/fixtures/minimal.jpg"),
            )
            .unwrap();
        }
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let thumb_dir = dir.path().join("thumbs");
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        ScanService::new(catalog.pool().clone(), thumb_dir.clone())
            .scan_root(root.id, &ScanControl::noop())
            .await
            .unwrap();

        let assets = AssetRepo::new(catalog.pool().clone());
        let first = assets
            .find_by_path(root.id, "first.jpg")
            .await
            .unwrap()
            .unwrap();
        let second = assets
            .find_by_path(root.id, "second.jpg")
            .await
            .unwrap()
            .unwrap();
        AssetMetaRepo::new(catalog.pool().clone())
            .upsert(&AssetMeta {
                asset_id: first.id,
                capture_at: Some(1_700_000_000),
                camera: None,
                lens: None,
                rating: Some(5),
                latitude: None,
                longitude: None,
                keywords_json: None,
            })
            .await
            .unwrap();
        AssetMetaRepo::new(catalog.pool().clone())
            .upsert(&AssetMeta {
                asset_id: second.id,
                capture_at: Some(1_600_000_000),
                camera: None,
                lens: None,
                rating: Some(1),
                latitude: None,
                longitude: None,
                keywords_json: None,
            })
            .await
            .unwrap();

        let query = QueryService::new(catalog.pool().clone(), thumb_dir);
        for sort in ["rating:desc", "path:desc", "date:desc"] {
            let page = query
                .query(&AssetFilter::default(), sort, 0, 10)
                .await
                .unwrap();
            assert_eq!(page.total, 2);
            assert_eq!(page.items.len(), 2);
        }
    }

    #[tokio::test]
    async fn batch_apply_meta_without_rating_refreshes_metadata() {
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        std::fs::write(
            photos.join("refresh.jpg"),
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let thumb_dir = dir.path().join("thumbs");
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        ScanService::new(catalog.pool().clone(), thumb_dir.clone())
            .scan_root(root.id, &ScanControl::noop())
            .await
            .unwrap();
        let asset = AssetRepo::new(catalog.pool().clone())
            .find_by_path(root.id, "refresh.jpg")
            .await
            .unwrap()
            .unwrap();
        let query = QueryService::new(catalog.pool().clone(), thumb_dir);
        let count = query
            .batch_apply_meta(&[asset.id], AssetMetaPatch { rating: None })
            .await
            .unwrap();
        assert_eq!(count, 1);
        let detail = query.get_detail(asset.id).await.unwrap();
        assert!(detail.meta.is_some());
    }

    #[tokio::test]
    async fn apply_meta_patch_uses_read_only_workspace_settings() {
        use crate::workspace::WorkspaceMediaSettings;

        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        std::fs::write(
            photos.join("readonly.jpg"),
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let thumb_dir = dir.path().join("thumbs");
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        ScanService::new(catalog.pool().clone(), thumb_dir.clone())
            .scan_root(root.id, &ScanControl::noop())
            .await
            .unwrap();
        let asset = AssetRepo::new(catalog.pool().clone())
            .find_by_path(root.id, "readonly.jpg")
            .await
            .unwrap()
            .unwrap();
        let before = std::fs::read(photos.join("readonly.jpg")).unwrap();
        let query = QueryService::with_media_settings(
            catalog.pool().clone(),
            thumb_dir,
            WorkspaceMediaSettings {
                read_only: true,
                workspace_xmp_dir: dir.path().join("xmp"),
            },
        );
        query
            .apply_meta_patch(asset.id, AssetMetaPatch { rating: Some(3) })
            .await
            .unwrap();
        let after = std::fs::read(photos.join("readonly.jpg")).unwrap();
        assert_eq!(before, after);
    }

    #[tokio::test]
    async fn query_meta_search_matches_lens_field() {
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        std::fs::write(
            photos.join("lens.jpg"),
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let thumb_dir = dir.path().join("thumbs");
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        ScanService::new(catalog.pool().clone(), thumb_dir.clone())
            .scan_root(root.id, &ScanControl::noop())
            .await
            .unwrap();
        let asset = AssetRepo::new(catalog.pool().clone())
            .find_by_path(root.id, "lens.jpg")
            .await
            .unwrap()
            .unwrap();
        AssetMetaRepo::new(catalog.pool().clone())
            .upsert(&AssetMeta {
                asset_id: asset.id,
                capture_at: None,
                camera: None,
                lens: Some("Sigma 35mm".into()),
                rating: None,
                latitude: None,
                longitude: None,
                keywords_json: None,
            })
            .await
            .unwrap();
        let query = QueryService::new(catalog.pool().clone(), thumb_dir);
        let result = query
            .query(
                &AssetFilter {
                    meta_search: Some("Sigma".into()),
                    ..Default::default()
                },
                "date:desc",
                0,
                10,
            )
            .await
            .unwrap();
        assert_eq!(result.total, 1);
    }

    #[tokio::test]
    async fn get_detail_uses_existing_index_without_repair() {
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        std::fs::write(
            photos.join("detail.jpg"),
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let thumb_dir = dir.path().join("thumbs");
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        ScanService::new(catalog.pool().clone(), thumb_dir.clone())
            .scan_root(root.id, &ScanControl::noop())
            .await
            .unwrap();
        let assets = AssetRepo::new(catalog.pool().clone());
        let asset = assets
            .find_by_path(root.id, "detail.jpg")
            .await
            .unwrap()
            .unwrap();
        let query = QueryService::new(catalog.pool().clone(), thumb_dir);
        let detail = query.get_detail(asset.id).await.unwrap();
        assert!(detail.asset.indexed_mtime_ns.is_some());
    }

    #[tokio::test]
    async fn batch_apply_meta_skips_duplicate_refresh_for_empty_ids() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let query = QueryService::new(catalog.pool().clone(), dir.path().join("thumbs"));
        let count = query
            .batch_apply_meta(&[], AssetMetaPatch { rating: Some(2) })
            .await
            .unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn resolve_filter_skips_empty_tag_ids() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let query = QueryService::new(catalog.pool().clone(), dir.path().join("thumbs"));
        let total = query
            .count(&AssetFilter {
                tag_ids: Some(vec![]),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(total, 0);
    }

    #[tokio::test]
    async fn query_service_errors_after_pool_close() {
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        std::fs::write(
            photos.join("closed.jpg"),
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let pool = catalog.pool().clone();
        let thumb_dir = dir.path().join("thumbs");
        let roots = SourceRootRepo::new(pool.clone());
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        ScanService::new(pool.clone(), thumb_dir.clone())
            .scan_root(root.id, &ScanControl::noop())
            .await
            .unwrap();
        let assets = AssetRepo::new(pool.clone());
        let asset = assets
            .find_by_path(root.id, "closed.jpg")
            .await
            .unwrap()
            .unwrap();
        let tag_id = TagRepo::new(pool.clone())
            .create_tag("closed", None, None)
            .await
            .unwrap();
        let query = QueryService::new(pool.clone(), thumb_dir);
        pool.close().await;
        assert!(query.count(&AssetFilter::default()).await.is_err());
        assert!(query
            .count(&AssetFilter {
                tag_ids: Some(vec![tag_id]),
                ..Default::default()
            })
            .await
            .is_err());
        assert!(query
            .query(&AssetFilter::default(), "date:desc", 0, 10)
            .await
            .is_err());
        assert!(query
            .query(
                &AssetFilter {
                    tag_ids: Some(vec![tag_id]),
                    ..Default::default()
                },
                "date:desc",
                0,
                10,
            )
            .await
            .is_err());
        assert!(query.get_detail(asset.id).await.is_err());
        assert!(query
            .apply_meta_patch(asset.id, AssetMetaPatch { rating: Some(1) })
            .await
            .is_err());
        assert!(query
            .batch_apply_meta(&[asset.id], AssetMetaPatch { rating: Some(1) })
            .await
            .is_err());
        assert!(query.batch_append_tags(&[asset.id], tag_id).await.is_err());
        assert!(query.batch_remove_tags(&[asset.id], tag_id).await.is_err());
    }

    #[tokio::test]
    async fn query_skips_empty_optional_filters_via_query() {
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        std::fs::write(
            photos.join("solo.jpg"),
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let thumb_dir = dir.path().join("thumbs");
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        ScanService::new(catalog.pool().clone(), thumb_dir.clone())
            .scan_root(root.id, &ScanControl::noop())
            .await
            .unwrap();
        let query = QueryService::new(catalog.pool().clone(), thumb_dir);
        for filter in [
            AssetFilter {
                sync_states: Some(vec![]),
                ..Default::default()
            },
            AssetFilter {
                album_ids: Some(vec![]),
                ..Default::default()
            },
            AssetFilter {
                meta_search: Some("   ".into()),
                ..Default::default()
            },
        ] {
            let page = query.query(&filter, "date:desc", 0, 10).await.unwrap();
            assert_eq!(page.total, 1);
        }
    }

    #[tokio::test]
    async fn query_filters_by_multiple_tag_ids() {
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        for name in ["one.jpg", "two.jpg"] {
            std::fs::write(
                photos.join(name),
                include_bytes!("../../tests/fixtures/minimal.jpg"),
            )
            .unwrap();
        }
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let thumb_dir = dir.path().join("thumbs");
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        ScanService::new(catalog.pool().clone(), thumb_dir.clone())
            .scan_root(root.id, &ScanControl::noop())
            .await
            .unwrap();
        let assets = AssetRepo::new(catalog.pool().clone());
        let one = assets
            .find_by_path(root.id, "one.jpg")
            .await
            .unwrap()
            .unwrap();
        let two = assets
            .find_by_path(root.id, "two.jpg")
            .await
            .unwrap()
            .unwrap();
        let tag_repo = TagRepo::new(catalog.pool().clone());
        let tag_a = tag_repo.create_tag("alpha", None, None).await.unwrap();
        let tag_b = tag_repo.create_tag("beta", None, None).await.unwrap();
        tag_repo
            .append_tag_id_to_assets(&[one.id], tag_a)
            .await
            .unwrap();
        tag_repo
            .append_tag_id_to_assets(&[two.id], tag_b)
            .await
            .unwrap();
        let query = QueryService::new(catalog.pool().clone(), thumb_dir);
        let result = query
            .query(
                &AssetFilter {
                    tag_ids: Some(vec![tag_a, tag_b]),
                    ..Default::default()
                },
                "date:desc",
                0,
                10,
            )
            .await
            .unwrap();
        assert_eq!(result.total, 2);
    }

    #[tokio::test]
    async fn query_meta_search_escapes_wildcard_chars() {
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        std::fs::write(
            photos.join("50%_x.jpg"),
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();
        std::fs::write(
            photos.join("other.jpg"),
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let thumb_dir = dir.path().join("thumbs");
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        ScanService::new(catalog.pool().clone(), thumb_dir.clone())
            .scan_root(root.id, &ScanControl::noop())
            .await
            .unwrap();
        let query = QueryService::new(catalog.pool().clone(), thumb_dir);
        let result = query
            .query(
                &AssetFilter {
                    meta_search: Some("50%_x".into()),
                    ..Default::default()
                },
                "date:desc",
                0,
                10,
            )
            .await
            .unwrap();
        assert_eq!(result.total, 1);
        assert_eq!(result.items[0].file_name, "50%_x.jpg");
    }

    #[tokio::test]
    async fn count_expands_tag_descendants() {
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        std::fs::write(
            photos.join("child.jpg"),
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let thumb_dir = dir.path().join("thumbs");
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        ScanService::new(catalog.pool().clone(), thumb_dir.clone())
            .scan_root(root.id, &ScanControl::noop())
            .await
            .unwrap();
        let assets = AssetRepo::new(catalog.pool().clone());
        let asset = assets
            .find_by_path(root.id, "child.jpg")
            .await
            .unwrap()
            .unwrap();
        let tag_repo = TagRepo::new(catalog.pool().clone());
        let parent_id = tag_repo.create_tag("travel", None, None).await.unwrap();
        let child_id = tag_repo
            .create_tag("japan", Some(parent_id), None)
            .await
            .unwrap();
        tag_repo
            .append_tag_id_to_assets(&[asset.id], child_id)
            .await
            .unwrap();
        let query = QueryService::new(catalog.pool().clone(), thumb_dir);
        let total = query
            .count(&AssetFilter {
                tag_ids: Some(vec![parent_id]),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(total, 1);
    }

    #[tokio::test]
    async fn get_detail_propagates_index_spawn_panic() {
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        std::fs::write(
            photos.join("panic.jpg"),
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let thumb_dir = dir.path().join("thumbs");
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        let assets = AssetRepo::new(catalog.pool().clone());
        let asset = assets
            .upsert_asset(crate::catalog::repo::UpsertAssetInput {
                root_id: root.id,
                rel_path: "panic.jpg".into(),
                file_name: "panic.jpg".into(),
                ext: "jpg".into(),
                kind: "image".into(),
                size: 1,
                mtime_ns: 99,
                sync_state: "ok",
            })
            .await
            .unwrap();
        std::env::set_var("MEMHG_TEST_INDEX_ON_DISK_PANIC", "1");
        let query = QueryService::new(catalog.pool().clone(), thumb_dir);
        let err = query.get_detail(asset.id).await.unwrap_err();
        assert!(err.to_string().contains("index on disk panic"));
    }

    #[tokio::test]
    async fn apply_meta_patch_without_rating_updates_metadata() {
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        std::fs::write(
            photos.join("norating.jpg"),
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let thumb_dir = dir.path().join("thumbs");
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        ScanService::new(catalog.pool().clone(), thumb_dir.clone())
            .scan_root(root.id, &ScanControl::noop())
            .await
            .unwrap();
        let asset = AssetRepo::new(catalog.pool().clone())
            .find_by_path(root.id, "norating.jpg")
            .await
            .unwrap()
            .unwrap();
        let query = QueryService::new(catalog.pool().clone(), thumb_dir);
        query
            .apply_meta_patch(asset.id, AssetMetaPatch { rating: None })
            .await
            .unwrap();
    }
}
