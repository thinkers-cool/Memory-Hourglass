use crate::activity::revert::UndoContext;
use crate::activity::ActivityEntry;
use crate::commands::context::{begin_command, finish_command};
use crate::error::Result;
use crate::state::AppState;
use tauri::State;

#[tauri::command] pub async fn query_asset_activity(
    asset_id: i64,
    offset: Option<i64>,
    limit: Option<i64>,
    state: State<'_, AppState>,
) -> Result<Vec<ActivityEntry>> {
    let correlation_id = begin_command("query_asset_activity");
    let result = state
        .with_active(|ws| async move {
            ws.activity
                .repo()
                .list_for_asset(asset_id, offset.unwrap_or(0), limit.unwrap_or(50))
                .await
        })
        .await;
    match &result {
        Ok(_) => finish_command("query_asset_activity", &correlation_id),
        Err(error) => crate::commands::context::fail_command(
            "query_asset_activity",
            &correlation_id,
            &error.to_string(),
        ),
    }
    result
}

#[tauri::command] pub async fn undo_activity(activity_id: i64, state: State<'_, AppState>) -> Result<i64> {
    let correlation_id = begin_command("undo_activity");
    let correlation_for_undo = correlation_id.clone();
    let result = state
        .with_active(|ws| async move {
            let undo_ctx = UndoContext {
                pool: ws.catalog.pool().clone(),
                thumb_dir: ws.thumb_dir.clone(),
                read_only: ws.media_settings.read_only,
                correlation_id: Some(correlation_for_undo),
            };
            crate::activity::revert::undo_activity(&undo_ctx, activity_id).await
        })
        .await;
    match &result {
        Ok(_) => finish_command("undo_activity", &correlation_id),
        Err(error) => crate::commands::context::fail_command(
            "undo_activity",
            &correlation_id,
            &error.to_string(),
        ),
    }
    result
}
