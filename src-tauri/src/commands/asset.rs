use crate::activity::record::{
    record_batch_metadata_changed, record_metadata_changed, record_purged,
    record_restored, record_soft_deleted,
};
use crate::catalog::models::{AssetDetail, AssetMetaPatch};
use crate::catalog::repo::{AssetMetaRepo, AssetRepo};
use crate::commands::context::begin_command;
use crate::error::{AppError, Result};
use crate::message::{emit_message, MessageEnvelope};
use crate::state::AppState;
use serde_json::json;
use tauri::{AppHandle, Runtime, State};

#[tauri::command] pub async fn get_asset(id: i64, state: State<'_, AppState>) -> Result<AssetDetail> {
    let _correlation_id = begin_command("get_asset");
    state
        .with_active(|ws| async move { ws.query_service().get_detail(id).await })
        .await
}

#[tauri::command] pub async fn update_asset_meta<R: Runtime>(
    id: i64,
    patch: AssetMetaPatch,
    app: AppHandle<R>,
    state: State<'_, AppState>,
) -> Result<AssetDetail> {
    let correlation_id = begin_command("update_asset_meta");
    let correlation_for_ws = correlation_id.clone();
    let result = state
        .with_active(|ws| async move {
            let meta_repo = AssetMetaRepo::new(ws.catalog.pool().clone());
            let before_meta = meta_repo.get(id).await?;
            let before = AssetMetaPatch {
                rating: before_meta.and_then(|m| m.rating),
            };
            let detail = ws.query_service().apply_meta_patch(id, patch).await?;
            let after = AssetMetaPatch {
                rating: detail.meta.as_ref().and_then(|m| m.rating),
            };
            let activity_id = record_metadata_changed(
                &ws.activity,
                Some(&correlation_for_ws),
                id,
                detail.asset.root_id,
                &detail.asset.rel_path,
                &before,
                &after,
            )
            .await?;
            Ok((detail, activity_id))
        })
        .await?;

    let (detail, activity_id) = result;
    emit_message(
        &app,
        MessageEnvelope::success("library:notification.rated")
            .with_params(json!({ "count": 1 }))
            .with_correlation(correlation_id)
            .with_undo(activity_id),
    );
    Ok(detail)
}

#[tauri::command] pub async fn soft_delete_assets<R: Runtime>(
    ids: Vec<i64>,
    app: AppHandle<R>,
    state: State<'_, AppState>,
) -> Result<u64> {
    let correlation_id = begin_command("soft_delete_assets");
    let correlation_for_ws = correlation_id.clone();
    let result = state
        .with_active(|ws| async move {
            let at = chrono::Utc::now().timestamp();
            let count = AssetRepo::new(ws.catalog.pool().clone())
                .soft_delete(&ids, at)
                .await?;
            ws.link.refresh_duplicate_flags().await?;
            let activity_id = record_soft_deleted(&ws.activity, Some(&correlation_for_ws), &ids, at)
                .await?;
            Ok((count, activity_id))
        })
        .await?;

    let (count, activity_id) = result;
    if count > 0 {
        emit_message(
            &app,
            MessageEnvelope::success("library:notification.deleted")
                .with_params(json!({ "count": count }))
                .with_correlation(correlation_id)
                .with_undo(activity_id),
        );
    }
    Ok(count)
}

#[tauri::command] pub async fn batch_update_asset_meta<R: Runtime>(
    ids: Vec<i64>,
    patch: AssetMetaPatch,
    app: AppHandle<R>,
    state: State<'_, AppState>,
) -> Result<u64> {
    let correlation_id = begin_command("batch_update_asset_meta");
    let correlation_for_ws = correlation_id.clone();
    let result = state
        .with_active(|ws| async move {
            let pool = ws.catalog.pool().clone();
            let meta_repo = AssetMetaRepo::new(pool.clone());
            let assets = AssetRepo::new(pool.clone());
            let mut items = Vec::new();
            for asset_id in &ids {
                let asset = assets.get_asset(*asset_id).await?;
                let before_meta = meta_repo.get(*asset_id).await?;
                let before = AssetMetaPatch {
                    rating: before_meta.and_then(|m| m.rating),
                };
                items.push((
                    *asset_id,
                    asset.root_id,
                    asset.rel_path.clone(),
                    before,
                    patch.clone(),
                ));
            }
            let count = ws
                .query_service()
                .batch_apply_meta(&ids, patch)
                .await?;
            let activity_id = record_batch_metadata_changed(
                &ws.activity,
                Some(&correlation_for_ws),
                &items,
            )
            .await?;
            Ok((count, activity_id))
        })
        .await?;

    let (count, activity_id) = result;
    if count > 0 {
        emit_message(
            &app,
            MessageEnvelope::success("library:notification.rated")
                .with_params(json!({ "count": count }))
                .with_correlation(correlation_id)
                .with_undo(activity_id),
        );
    }
    Ok(count)
}

#[tauri::command] pub async fn restore_assets(ids: Vec<i64>, state: State<'_, AppState>) -> Result<u64> {
    let correlation_id = begin_command("restore_assets");
    state
        .with_active(|ws| async move {
            let assets = AssetRepo::new(ws.catalog.pool().clone());
            let deleted_map = assets.get_deleted_at_map(&ids).await?;
            let count = assets.restore_assets(&ids).await?;
            ws.link.refresh_duplicate_flags().await?;
            if count > 0 { let first_deleted_at = deleted_map.values().next().copied().unwrap_or(chrono::Utc::now().timestamp()); record_restored(&ws.activity, Some(&correlation_id), &ids, first_deleted_at).await?; }
            Ok(count)
        })
        .await
}

#[tauri::command] pub async fn purge_delete(
    ids: Vec<i64>,
    confirm_token: String,
    state: State<'_, AppState>,
) -> Result<u64> {
    let correlation_id = begin_command("purge_delete");
    if confirm_token != "DELETE" {
        return Err(AppError::InvalidInput(
            "confirm_token must be DELETE".into(),
        ));
    }
    state
        .with_active(|ws| async move {
            if ws.info.read_only {
                return Err(AppError::InvalidInput(
                    "cannot purge source files in a read-only workspace".into(),
                ));
            }
            let count = AssetRepo::new(ws.catalog.pool().clone())
                .purge_assets(&ids, &ws.paths, ws.media_settings.read_only)
                .await?;
            ws.link.refresh_duplicate_flags().await?;
            if count > 0 { record_purged(&ws.activity, Some(&correlation_id), &ids).await?; }
            Ok(count)
        })
        .await
}
