use crate::commands::context::trace_command;
use crate::error::{AppError, Result};
use crate::query::{AssetFilter, QueryResult};
use crate::state::AppState;
use tauri::State;

const MAX_QUERY_LIMIT: i64 = 500;

#[tauri::command]
pub async fn query_assets(
    filter: AssetFilter,
    sort: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
    state: State<'_, AppState>,
) -> Result<QueryResult> {
    trace_command("query_assets", || async move {
        state
            .with_active(|ws| async move {
                ws.query_service()
                    .query(
                        &filter,
                        sort.as_deref()
                            .ok_or_else(|| AppError::InvalidInput("sort is required".into()))?,
                        offset.unwrap_or(0).max(0),
                        limit.unwrap_or(200).clamp(1, MAX_QUERY_LIMIT),
                    )
                    .await
            })
            .await
    })
    .await
}

#[tauri::command]
pub async fn count_assets(filter: AssetFilter, state: State<'_, AppState>) -> Result<i64> {
    trace_command("count_assets", || async move {
        state
            .with_active(|ws| async move { ws.query_service().count(&filter).await })
            .await
    })
    .await
}
