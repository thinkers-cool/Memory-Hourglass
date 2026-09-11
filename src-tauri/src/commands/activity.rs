use crate::activity::revert::UndoContext;
use crate::activity::ActivityEntry;
use crate::commands::context::trace_command;
use crate::error::Result;
use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub async fn query_asset_activity(
    asset_id: i64,
    offset: Option<i64>,
    limit: Option<i64>,
    state: State<'_, AppState>,
) -> Result<Vec<ActivityEntry>> {
    trace_command("query_asset_activity", |_correlation_id| async move {
        state
            .with_active(|ws| async move {
                ws.activity
                    .repo()
                    .list_for_asset(asset_id, offset.unwrap_or(0), limit.unwrap_or(50))
                    .await
            })
            .await
    })
    .await
}

#[tauri::command]
pub async fn undo_activity(activity_id: i64, state: State<'_, AppState>) -> Result<i64> {
    trace_command("undo_activity", |correlation_id| async move {
        let correlation_for_undo = correlation_id.clone();
        state
            .with_active(|ws| async move {
                let undo_ctx = UndoContext {
                    pool: ws.catalog.pool().clone(),
                    thumb_dir: ws.thumb_dir.clone(),
                    read_only: ws.media_settings.read_only,
                    correlation_id: Some(correlation_for_undo),
                };
                crate::activity::revert::undo_activity(&undo_ctx, activity_id).await
            })
            .await
    })
    .await
}
