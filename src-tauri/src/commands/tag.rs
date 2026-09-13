use crate::activity::record::{record_tags_added, record_tags_removed};
use crate::catalog::repo::TagRepo;
use crate::commands::context::{trace_command, trace_command_with_id};
use crate::error::Result;
use crate::message::{emit_message, MessageEnvelope};
use crate::query::tag_keywords::sync_assets_tag_keywords;
use crate::state::AppState;
use serde::Serialize;
use serde_json::json;
use tauri::{AppHandle, Runtime, State};

#[derive(Serialize)]
pub struct TagDto {
    pub id: i64,
    pub name: String,
    pub parent_id: Option<i64>,
    pub color: Option<String>,
    pub asset_count: i64,
}

#[tauri::command]
pub async fn list_tags(state: State<'_, AppState>) -> Result<Vec<TagDto>> {
    trace_command("list_tags", || async move {
        state
            .with_active(|ws| async move {
                let rows = TagRepo::new(ws.catalog.pools().clone())
                    .list_tag_rows()
                    .await?;
                Ok(rows
                    .into_iter()
                    .map(|row| TagDto {
                        id: row.id,
                        name: row.name,
                        parent_id: row.parent_id,
                        color: row.color,
                        asset_count: row.asset_count,
                    })
                    .collect())
            })
            .await
    })
    .await
}

#[tauri::command]
pub async fn create_tag(
    name: String,
    parent_id: Option<i64>,
    color: Option<String>,
    state: State<'_, AppState>,
) -> Result<TagDto> {
    trace_command("create_tag", || async move {
        state
            .with_active(|ws| async move {
                let repo = TagRepo::new(ws.catalog.pools().clone());
                let id = repo.create_tag(&name, parent_id, color.as_deref()).await?;
                Ok(TagDto {
                    id,
                    name,
                    parent_id,
                    color,
                    asset_count: 0,
                })
            })
            .await
    })
    .await
}

#[tauri::command]
pub async fn update_tag(
    id: i64,
    name: String,
    color: Option<String>,
    state: State<'_, AppState>,
) -> Result<TagDto> {
    trace_command("update_tag", || async move {
        state
            .with_active(|ws| async move {
                let pools = ws.catalog.pools().clone();
                let repo = TagRepo::new(pools.clone());
                let asset_ids = repo.list_asset_ids_for_tag(id).await?;
                let row = repo.update_tag(id, &name, color.as_deref()).await?;
                sync_assets_tag_keywords(&pools, &asset_ids, &ws.media_settings).await?;
                Ok(TagDto {
                    id: row.id,
                    name: row.name,
                    parent_id: row.parent_id,
                    color: row.color,
                    asset_count: row.asset_count,
                })
            })
            .await
    })
    .await
}

#[tauri::command]
pub async fn delete_tag(id: i64, state: State<'_, AppState>) -> Result<()> {
    trace_command("delete_tag", || async move {
        state
            .with_active(|ws| async move {
                let pools = ws.catalog.pools().clone();
                let repo = TagRepo::new(pools.clone());
                let asset_ids = repo.list_asset_ids_for_tag(id).await?;
                repo.delete_tag(id).await?;
                sync_assets_tag_keywords(&pools, &asset_ids, &ws.media_settings).await?;
                Ok(())
            })
            .await
    })
    .await
}

#[tauri::command]
pub async fn batch_append_tags<R: Runtime>(
    asset_ids: Vec<i64>,
    tag_id: i64,
    app: AppHandle<R>,
    state: State<'_, AppState>,
) -> Result<u64> {
    trace_command_with_id("batch_append_tags", |correlation_id| async move {
        let correlation_for_ws = correlation_id.clone();
        let count = state
            .with_active(|ws| async move {
                let count = ws
                    .query_service()
                    .batch_append_tags(&asset_ids, tag_id)
                    .await?;
                if count > 0 {
                    record_tags_added(&ws.activity, Some(&correlation_for_ws), &asset_ids, tag_id)
                        .await?;
                }
                Ok(count)
            })
            .await?;

        if count > 0 {
            emit_message(
                &app,
                MessageEnvelope::success("library:notification.tagged")
                    .with_params(json!({ "count": count }))
                    .with_correlation(correlation_id),
            );
        }
        Ok(count)
    })
    .await
}

#[tauri::command]
pub async fn batch_remove_tags<R: Runtime>(
    asset_ids: Vec<i64>,
    tag_id: i64,
    app: AppHandle<R>,
    state: State<'_, AppState>,
) -> Result<u64> {
    trace_command_with_id("batch_remove_tags", |correlation_id| async move {
        let correlation_for_ws = correlation_id.clone();
        let count = state
            .with_active(|ws| async move {
                let count = ws
                    .query_service()
                    .batch_remove_tags(&asset_ids, tag_id)
                    .await?;
                if count > 0 {
                    record_tags_removed(
                        &ws.activity,
                        Some(&correlation_for_ws),
                        &asset_ids,
                        tag_id,
                    )
                    .await?;
                }
                Ok(count)
            })
            .await?;

        if count > 0 {
            emit_message(
                &app,
                MessageEnvelope::success("library:notification.tagRemoved")
                    .with_params(json!({ "count": count }))
                    .with_correlation(correlation_id),
            );
        }
        Ok(count)
    })
    .await
}
