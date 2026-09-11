use crate::activity::models::actor;
use crate::activity::models::event_type;
use crate::activity::models::ActivityInput;
use crate::activity::recorder::ActivityRecorder;
use crate::catalog::models::AssetMetaPatch;
use crate::catalog::repo::AssetRepo;
use crate::error::{AppError, Result};
use crate::link::LinkService;
use crate::query::QueryService;
use serde::Deserialize;
use serde_json::json;
use sqlx::sqlite::SqlitePool;
use std::path::PathBuf;

pub struct UndoContext {
    pub pool: SqlitePool,
    pub thumb_dir: PathBuf,
    pub read_only: bool,
    pub correlation_id: Option<String>,
}

pub async fn undo_activity(ctx: &UndoContext, activity_id: i64) -> Result<i64> {
    let recorder = ActivityRecorder::new(ctx.pool.clone());
    let entry = recorder.repo().get(activity_id).await?;

    if entry.undone_at.is_some() {
        return Err(AppError::Conflict("activity already undone".into()));
    }
    if entry.revert_json.is_none() {
        return Err(AppError::InvalidInput("activity is not reversible".into()));
    }

    let revert = entry
        .revert_json
        .as_deref()
        .expect("revert_json present after check");

    let media_settings = crate::workspace::WorkspaceMediaSettings {
        read_only: ctx.read_only,
        workspace_xmp_dir: PathBuf::new(),
    };

    match entry.event_type.as_str() {
        event_type::ASSET_METADATA_CHANGED => {
            let payload: MetadataRevert = serde_json::from_str(revert)?;
            let query = QueryService::with_media_settings(
                ctx.pool.clone(),
                ctx.thumb_dir.clone(),
                media_settings.clone(),
            );
            for item in payload.items {
                query.apply_meta_patch(item.asset_id, item.patch).await?;
            }
        }
        event_type::ASSET_SOFT_DELETED => {
            let payload: SoftDeleteRevert = serde_json::from_str(revert)?;
            AssetRepo::new(ctx.pool.clone())
                .restore_assets(&payload.asset_ids)
                .await?;
        }
        event_type::ASSET_RESTORED => {
            let payload: RestoreRevert = serde_json::from_str(revert)?;
            let assets = AssetRepo::new(ctx.pool.clone());
            for item in payload.items()? {
                assets
                    .soft_delete(&[item.asset_id], item.deleted_at)
                    .await?;
            }
        }
        event_type::ASSET_TAGS_ADDED => {
            let payload: TagsRevert = serde_json::from_str(revert)?;
            QueryService::with_media_settings(
                ctx.pool.clone(),
                ctx.thumb_dir.clone(),
                media_settings,
            )
            .batch_remove_tags(&payload.asset_ids, payload.tag_id)
            .await?;
        }
        event_type::ASSET_TAGS_REMOVED => {
            let payload: TagsRevert = serde_json::from_str(revert)?;
            QueryService::with_media_settings(
                ctx.pool.clone(),
                ctx.thumb_dir.clone(),
                media_settings,
            )
            .batch_append_tags(&payload.asset_ids, payload.tag_id)
            .await?;
        }
        other => {
            return Err(AppError::InvalidInput(format!(
                "cannot undo event type {}",
                other
            )));
        }
    }

    LinkService::new(ctx.pool.clone())
        .refresh_duplicate_flags()
        .await?;

    let undo_event = ActivityInput {
        event_type: format!("{}.undone", entry.event_type),
        actor: actor::USER.to_string(),
        correlation_id: ctx.correlation_id.clone(),
        subject_type: entry.subject_type.clone(),
        subject_id: entry.subject_id,
        subject_key: entry.subject_key.clone(),
        summary: Some(format!("undone activity {}", activity_id)),
        payload_json: json!({ "undone_activity_id": activity_id }).to_string(),
        revert_json: None,
    };
    let mut tx = ctx.pool.begin().await?;
    let undo_id = recorder.append_in_tx(&mut tx, undo_event).await?;
    recorder
        .repo()
        .mark_undone(&mut tx, activity_id, undo_id)
        .await?;
    tx.commit().await?;

    Ok(undo_id)
}

#[derive(Deserialize)]
struct MetadataRevertItem {
    asset_id: i64,
    patch: AssetMetaPatch,
}

#[derive(Deserialize)]
struct MetadataRevert {
    items: Vec<MetadataRevertItem>,
}

#[derive(Deserialize)]
struct SoftDeleteRevert {
    asset_ids: Vec<i64>,
}

#[derive(Clone, Deserialize)]
struct RestoreRevertItem {
    asset_id: i64,
    deleted_at: i64,
}

#[derive(Deserialize)]
struct RestoreRevert {
    asset_ids: Option<Vec<i64>>,
    deleted_at: Option<i64>,
    items: Option<Vec<RestoreRevertItem>>,
}

impl RestoreRevert {
    fn items(&self) -> Result<Vec<RestoreRevertItem>> {
        if let Some(items) = &self.items {
            return Ok(items.clone());
        }
        let asset_ids = self
            .asset_ids
            .clone()
            .ok_or_else(|| AppError::InvalidInput("invalid restore revert".into()))?;
        let deleted_at = self
            .deleted_at
            .ok_or_else(|| AppError::InvalidInput("invalid restore revert".into()))?;
        Ok(asset_ids
            .into_iter()
            .map(|asset_id| RestoreRevertItem {
                asset_id,
                deleted_at,
            })
            .collect())
    }
}

#[derive(Deserialize)]
struct TagsRevert {
    asset_ids: Vec<i64>,
    tag_id: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::activity::models::event_type;
    use crate::activity::models::ActivityInput;
    use crate::activity::recorder::ActivityRecorder;
    use crate::catalog::models::AssetMetaPatch;
    use crate::catalog::repo::{AssetRepo, TagRepo};
    use crate::catalog::Catalog;
    use crate::library::LibraryService;
    use crate::query::QueryService;
    use crate::scan::{ScanControl, ScanService};
    use tempfile::tempdir;

    async fn seeded_asset(
        pool: &sqlx::SqlitePool,
        thumb_dir: &PathBuf,
        workspace_dir: &std::path::Path,
    ) -> (i64, i64) {
        let photos = workspace_dir.join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        std::fs::write(
            photos.join("item.jpg"),
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();
        let library = LibraryService::new(pool.clone(), workspace_dir.to_path_buf());
        let root = library
            .add_local_root(photos.to_str().unwrap())
            .await
            .unwrap();
        ScanService::new(pool.clone(), thumb_dir.clone())
            .scan_root(root.id, &ScanControl::noop())
            .await
            .unwrap();
        let listed = QueryService::new(pool.clone(), thumb_dir.clone())
            .query(&crate::query::AssetFilter::default(), "date:desc", 0, 10)
            .await
            .unwrap();
        let asset_id = listed.items[0].id;
        (asset_id, root.id)
    }

    fn undo_ctx(pool: sqlx::SqlitePool, thumb_dir: PathBuf) -> UndoContext {
        UndoContext {
            pool,
            thumb_dir,
            read_only: false,
            correlation_id: Some("undo-test".into()),
        }
    }

    #[tokio::test]
    async fn undo_rejects_already_undone_activity() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let pool = catalog.pool().clone();
        let thumb_dir = dir.path().join("thumbs");
        let (asset_id, _) = seeded_asset(&pool, &thumb_dir, dir.path()).await;
        let deleted_at = 1_700_000_000i64;
        AssetRepo::new(pool.clone())
            .soft_delete(&[asset_id], deleted_at)
            .await
            .unwrap();
        let recorder = ActivityRecorder::new(pool.clone());
        let activity_id = recorder
            .append(ActivityInput {
                event_type: event_type::ASSET_SOFT_DELETED.to_string(),
                actor: actor::USER.to_string(),
                correlation_id: None,
                subject_type: Some("asset".into()),
                subject_id: Some(asset_id),
                subject_key: None,
                summary: None,
                payload_json: "{}".into(),
                revert_json: Some(serde_json::json!({ "asset_ids": [asset_id] }).to_string()),
            })
            .await
            .unwrap();
        let ctx = undo_ctx(pool.clone(), thumb_dir);
        undo_activity(&ctx, activity_id).await.unwrap();
        let err = undo_activity(&ctx, activity_id).await.unwrap_err();
        assert!(matches!(err, AppError::Conflict(_)));
        assert!(err.to_string().contains("already undone"));
    }

    #[tokio::test]
    async fn undo_rejects_already_undone_and_non_reversible() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let pool = catalog.pool().clone();
        let thumb_dir = dir.path().join("thumbs");
        let recorder = ActivityRecorder::new(pool.clone());
        let activity_id = recorder
            .append(ActivityInput {
                event_type: "scan.completed".into(),
                actor: actor::USER.to_string(),
                correlation_id: None,
                subject_type: Some("root".into()),
                subject_id: Some(1),
                subject_key: None,
                summary: None,
                payload_json: "{}".into(),
                revert_json: None,
            })
            .await
            .unwrap();
        let ctx = undo_ctx(pool.clone(), thumb_dir);
        assert!(undo_activity(&ctx, activity_id).await.is_err());
    }

    #[tokio::test]
    async fn undo_soft_delete_restores_assets() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let pool = catalog.pool().clone();
        let thumb_dir = dir.path().join("thumbs");
        let (asset_id, _) = seeded_asset(&pool, &thumb_dir, dir.path()).await;
        let deleted_at = 1_700_000_000i64;
        AssetRepo::new(pool.clone())
            .soft_delete(&[asset_id], deleted_at)
            .await
            .unwrap();
        let recorder = ActivityRecorder::new(pool.clone());
        let activity_id = recorder
            .append(ActivityInput {
                event_type: event_type::ASSET_SOFT_DELETED.to_string(),
                actor: actor::USER.to_string(),
                correlation_id: None,
                subject_type: Some("asset".into()),
                subject_id: Some(asset_id),
                subject_key: None,
                summary: None,
                payload_json: "{}".into(),
                revert_json: Some(serde_json::json!({ "asset_ids": [asset_id] }).to_string()),
            })
            .await
            .unwrap();
        undo_activity(&undo_ctx(pool.clone(), thumb_dir), activity_id)
            .await
            .unwrap();
        let asset = AssetRepo::new(pool.clone())
            .get_asset(asset_id)
            .await
            .unwrap();
        assert!(asset.deleted_at.is_none());
    }

    #[tokio::test]
    async fn undo_restore_soft_deletes_again() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let pool = catalog.pool().clone();
        let thumb_dir = dir.path().join("thumbs");
        let (asset_id, _) = seeded_asset(&pool, &thumb_dir, dir.path()).await;
        let deleted_at = 1_700_000_000i64;
        let recorder = ActivityRecorder::new(pool.clone());
        let activity_id = recorder
            .append(ActivityInput {
                event_type: event_type::ASSET_RESTORED.to_string(),
                actor: actor::USER.to_string(),
                correlation_id: None,
                subject_type: Some("asset".into()),
                subject_id: Some(asset_id),
                subject_key: None,
                summary: None,
                payload_json: "{}".into(),
                revert_json: Some(
                    serde_json::json!({
                        "asset_ids": [asset_id],
                        "deleted_at": deleted_at
                    })
                    .to_string(),
                ),
            })
            .await
            .unwrap();
        undo_activity(&undo_ctx(pool.clone(), thumb_dir), activity_id)
            .await
            .unwrap();
        let deleted = AssetRepo::new(pool.clone())
            .get_deleted_at_map(&[asset_id])
            .await
            .unwrap();
        assert_eq!(deleted.get(&asset_id), Some(&deleted_at));
    }

    #[tokio::test]
    async fn undo_tag_changes_restore_prior_tags() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let pool = catalog.pool().clone();
        let thumb_dir = dir.path().join("thumbs");
        let (asset_id, _) = seeded_asset(&pool, &thumb_dir, dir.path()).await;
        let tag_repo = TagRepo::new(pool.clone());
        let tag_id = tag_repo.create_tag("trip", None, None).await.unwrap();
        let query = QueryService::new(pool.clone(), thumb_dir.clone());
        query.batch_append_tags(&[asset_id], tag_id).await.unwrap();
        let recorder = ActivityRecorder::new(pool.clone());
        let activity_id = recorder
            .append(ActivityInput {
                event_type: event_type::ASSET_TAGS_ADDED.to_string(),
                actor: actor::USER.to_string(),
                correlation_id: None,
                subject_type: Some("asset".into()),
                subject_id: Some(asset_id),
                subject_key: None,
                summary: None,
                payload_json: "{}".into(),
                revert_json: Some(
                    serde_json::json!({ "asset_ids": [asset_id], "tag_id": tag_id }).to_string(),
                ),
            })
            .await
            .unwrap();
        undo_activity(&undo_ctx(pool.clone(), thumb_dir), activity_id)
            .await
            .unwrap();
        let tag_ids = tag_repo.list_ids_for_asset(asset_id).await.unwrap();
        assert!(tag_ids.is_empty());
    }

    #[tokio::test]
    async fn undo_tag_removed_restores_tag() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let pool = catalog.pool().clone();
        let thumb_dir = dir.path().join("thumbs");
        let (asset_id, _) = seeded_asset(&pool, &thumb_dir, dir.path()).await;
        let tag_repo = TagRepo::new(pool.clone());
        let tag_id = tag_repo.create_tag("trip", None, None).await.unwrap();
        tag_repo
            .append_tag_id_to_assets(&[asset_id], tag_id)
            .await
            .unwrap();
        let recorder = ActivityRecorder::new(pool.clone());
        let activity_id = recorder
            .append(ActivityInput {
                event_type: event_type::ASSET_TAGS_REMOVED.to_string(),
                actor: actor::USER.to_string(),
                correlation_id: None,
                subject_type: Some("asset".into()),
                subject_id: Some(asset_id),
                subject_key: None,
                summary: None,
                payload_json: "{}".into(),
                revert_json: Some(
                    serde_json::json!({ "asset_ids": [asset_id], "tag_id": tag_id }).to_string(),
                ),
            })
            .await
            .unwrap();
        undo_activity(&undo_ctx(pool.clone(), thumb_dir), activity_id)
            .await
            .unwrap();
        let tag_ids = tag_repo.list_ids_for_asset(asset_id).await.unwrap();
        assert_eq!(tag_ids, vec![tag_id]);
    }

    #[tokio::test]
    async fn undo_metadata_change_restores_prior_rating() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let pool = catalog.pool().clone();
        let thumb_dir = dir.path().join("thumbs");
        let (asset_id, _) = seeded_asset(&pool, &thumb_dir, dir.path()).await;
        let query = QueryService::new(pool.clone(), thumb_dir.clone());
        query
            .apply_meta_patch(asset_id, AssetMetaPatch { rating: Some(5) })
            .await
            .unwrap();
        let recorder = ActivityRecorder::new(pool.clone());
        let activity_id = recorder
            .append(ActivityInput {
                event_type: event_type::ASSET_METADATA_CHANGED.to_string(),
                actor: actor::USER.to_string(),
                correlation_id: None,
                subject_type: Some("asset".into()),
                subject_id: Some(asset_id),
                subject_key: None,
                summary: None,
                payload_json: "{}".into(),
                revert_json: Some(
                    serde_json::json!({
                        "items": [{
                            "asset_id": asset_id,
                            "patch": { "rating": 1 }
                        }]
                    })
                    .to_string(),
                ),
            })
            .await
            .unwrap();
        undo_activity(&undo_ctx(pool.clone(), thumb_dir), activity_id)
            .await
            .unwrap();
        let detail = query.get_detail(asset_id).await.unwrap();
        assert_eq!(detail.meta.as_ref().and_then(|m| m.rating), Some(1));
    }

    #[tokio::test]
    async fn undo_rejects_unknown_event_type() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let pool = catalog.pool().clone();
        let thumb_dir = dir.path().join("thumbs");
        let recorder = ActivityRecorder::new(pool.clone());
        let activity_id = recorder
            .append(ActivityInput {
                event_type: "custom.event".into(),
                actor: actor::USER.to_string(),
                correlation_id: None,
                subject_type: Some("asset".into()),
                subject_id: Some(1),
                subject_key: None,
                summary: None,
                payload_json: "{}".into(),
                revert_json: Some("{}".into()),
            })
            .await
            .unwrap();
        let err = undo_activity(&undo_ctx(pool.clone(), thumb_dir), activity_id)
            .await
            .unwrap_err();
        assert!(err.to_string().contains("cannot undo"));
    }

    #[tokio::test]
    async fn undo_rejects_invalid_revert_json() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let pool = catalog.pool().clone();
        let thumb_dir = dir.path().join("thumbs");
        let (asset_id, _) = seeded_asset(&pool, &thumb_dir, dir.path()).await;
        let recorder = ActivityRecorder::new(pool.clone());
        let activity_id = recorder
            .append(ActivityInput {
                event_type: event_type::ASSET_METADATA_CHANGED.to_string(),
                actor: actor::USER.to_string(),
                correlation_id: None,
                subject_type: Some("asset".into()),
                subject_id: Some(asset_id),
                subject_key: None,
                summary: None,
                payload_json: "{}".into(),
                revert_json: Some("{not-json".into()),
            })
            .await
            .unwrap();
        let err = undo_activity(&undo_ctx(pool.clone(), thumb_dir), activity_id)
            .await
            .unwrap_err();
        assert!(!err.to_string().is_empty());
    }

    #[tokio::test]
    async fn undo_fails_when_pool_closed_before_restore() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let pool = catalog.pool().clone();
        let thumb_dir = dir.path().join("thumbs");
        let (asset_id, _) = seeded_asset(&pool, &thumb_dir, dir.path()).await;
        AssetRepo::new(pool.clone())
            .soft_delete(&[asset_id], 1)
            .await
            .unwrap();
        let recorder = ActivityRecorder::new(pool.clone());
        let activity_id = recorder
            .append(ActivityInput {
                event_type: event_type::ASSET_SOFT_DELETED.to_string(),
                actor: actor::USER.to_string(),
                correlation_id: None,
                subject_type: Some("asset".into()),
                subject_id: Some(asset_id),
                subject_key: None,
                summary: None,
                payload_json: "{}".into(),
                revert_json: Some(serde_json::json!({ "asset_ids": [asset_id] }).to_string()),
            })
            .await
            .unwrap();
        pool.close().await;
        assert!(undo_activity(&undo_ctx(pool, thumb_dir), activity_id)
            .await
            .is_err());
    }

    #[tokio::test]
    async fn undo_records_followup_activity_and_marks_original() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let pool = catalog.pool().clone();
        let thumb_dir = dir.path().join("thumbs");
        let (asset_id, _) = seeded_asset(&pool, &thumb_dir, dir.path()).await;
        let deleted_at = 1_700_000_001i64;
        AssetRepo::new(pool.clone())
            .soft_delete(&[asset_id], deleted_at)
            .await
            .unwrap();
        let recorder = ActivityRecorder::new(pool.clone());
        let activity_id = recorder
            .append(ActivityInput {
                event_type: event_type::ASSET_SOFT_DELETED.to_string(),
                actor: actor::USER.to_string(),
                correlation_id: None,
                subject_type: Some("asset".into()),
                subject_id: Some(asset_id),
                subject_key: None,
                summary: None,
                payload_json: "{}".into(),
                revert_json: Some(serde_json::json!({ "asset_ids": [asset_id] }).to_string()),
            })
            .await
            .unwrap();
        let undo_id = undo_activity(&undo_ctx(pool.clone(), thumb_dir), activity_id)
            .await
            .unwrap();
        let original = recorder.repo().get(activity_id).await.unwrap();
        assert!(original.undone_at.is_some());
        let undo_entry = recorder.repo().get(undo_id).await.unwrap();
        assert!(undo_entry.event_type.ends_with(".undone"));
    }

    #[tokio::test]
    async fn undo_metadata_fails_for_invalid_asset_id() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let pool = catalog.pool().clone();
        let thumb_dir = dir.path().join("thumbs");
        let recorder = ActivityRecorder::new(pool.clone());
        let activity_id = recorder
            .append(ActivityInput {
                event_type: event_type::ASSET_METADATA_CHANGED.to_string(),
                actor: actor::USER.to_string(),
                correlation_id: None,
                subject_type: Some("asset".into()),
                subject_id: Some(999_999),
                subject_key: None,
                summary: None,
                payload_json: "{}".into(),
                revert_json: Some(
                    serde_json::json!({
                        "items": [{
                            "asset_id": 999_999,
                            "patch": { "rating": 1 }
                        }]
                    })
                    .to_string(),
                ),
            })
            .await
            .unwrap();
        assert!(undo_activity(&undo_ctx(pool, thumb_dir), activity_id)
            .await
            .is_err());
    }

    #[tokio::test]
    async fn undo_restore_fails_when_pool_closed_before_soft_delete() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let pool = catalog.pool().clone();
        let thumb_dir = dir.path().join("thumbs");
        let (asset_id, _) = seeded_asset(&pool, &thumb_dir, dir.path()).await;
        let recorder = ActivityRecorder::new(pool.clone());
        let activity_id = recorder
            .append(ActivityInput {
                event_type: event_type::ASSET_RESTORED.to_string(),
                actor: actor::USER.to_string(),
                correlation_id: None,
                subject_type: Some("asset".into()),
                subject_id: Some(asset_id),
                subject_key: None,
                summary: None,
                payload_json: "{}".into(),
                revert_json: Some(
                    serde_json::json!({
                        "asset_ids": [asset_id],
                        "deleted_at": 1
                    })
                    .to_string(),
                ),
            })
            .await
            .unwrap();
        pool.close().await;
        assert!(undo_activity(&undo_ctx(pool, thumb_dir), activity_id)
            .await
            .is_err());
    }

    #[tokio::test]
    async fn undo_tags_added_fails_when_pool_closed_before_remove() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let pool = catalog.pool().clone();
        let thumb_dir = dir.path().join("thumbs");
        let (asset_id, _) = seeded_asset(&pool, &thumb_dir, dir.path()).await;
        let tag_id = TagRepo::new(pool.clone())
            .create_tag("trip", None, None)
            .await
            .unwrap();
        let recorder = ActivityRecorder::new(pool.clone());
        let activity_id = recorder
            .append(ActivityInput {
                event_type: event_type::ASSET_TAGS_ADDED.to_string(),
                actor: actor::USER.to_string(),
                correlation_id: None,
                subject_type: Some("asset".into()),
                subject_id: Some(asset_id),
                subject_key: None,
                summary: None,
                payload_json: "{}".into(),
                revert_json: Some(
                    serde_json::json!({ "asset_ids": [asset_id], "tag_id": tag_id }).to_string(),
                ),
            })
            .await
            .unwrap();
        pool.close().await;
        assert!(undo_activity(&undo_ctx(pool, thumb_dir), activity_id)
            .await
            .is_err());
    }

    #[tokio::test]
    async fn undo_soft_delete_fails_when_pool_closed_before_mark_undone() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let pool = catalog.pool().clone();
        let thumb_dir = dir.path().join("thumbs");
        let (asset_id, _) = seeded_asset(&pool, &thumb_dir, dir.path()).await;
        AssetRepo::new(pool.clone())
            .soft_delete(&[asset_id], 1)
            .await
            .unwrap();
        let recorder = ActivityRecorder::new(pool.clone());
        let activity_id = recorder
            .append(ActivityInput {
                event_type: event_type::ASSET_SOFT_DELETED.to_string(),
                actor: actor::USER.to_string(),
                correlation_id: None,
                subject_type: Some("asset".into()),
                subject_id: Some(asset_id),
                subject_key: None,
                summary: None,
                payload_json: "{}".into(),
                revert_json: Some(serde_json::json!({ "asset_ids": [asset_id] }).to_string()),
            })
            .await
            .unwrap();
        let ctx = undo_ctx(pool.clone(), thumb_dir);
        pool.close().await;
        assert!(undo_activity(&ctx, activity_id).await.is_err());
    }
}
