use crate::error::{AppError, Result};
use crate::query::{AssetFilter, QueryResult};
use crate::state::AppState;
use tauri::State;

#[tauri::command] pub async fn query_assets(
    filter: AssetFilter,
    sort: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
    state: State<'_, AppState>,
) -> Result<QueryResult> {
    state
        .with_active(|ws| async move {
            ws.query_service()
                .query(
                    &filter,
                    sort.as_deref()
                        .ok_or_else(|| AppError::InvalidInput("sort is required".into()))?,
                    offset.unwrap_or(0),
                    limit.unwrap_or(200),
                )
                .await
        })
        .await
}

#[tauri::command] pub async fn count_assets(filter: AssetFilter, state: State<'_, AppState>) -> Result<i64> {
    state
        .with_active(|ws| async move { ws.query_service().count(&filter).await })
        .await
}
