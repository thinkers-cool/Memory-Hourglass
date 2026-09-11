use crate::commands::context::trace_command;
use crate::error::Result;
use crate::state::AppState;
use crate::workspace::RecentWorkspace;
use crate::workspace::WorkspaceInfo;
use crate::workspace::WorkspaceService;
use tauri::State;

#[tauri::command]
pub async fn create_workspace(
    path: String,
    read_only: bool,
    state: State<'_, AppState>,
) -> Result<WorkspaceInfo> {
    trace_command("create_workspace", |_correlation_id| async move {
        let info = {
            let mut workspaces = state.workspaces.write().await;
            workspaces.create(std::path::Path::new(&path), read_only)?
        };
        state
            .open_workspace(std::path::Path::new(&info.path))
            .await?;
        Ok(info)
    })
    .await
}

#[tauri::command]
pub async fn open_workspace(path: String, state: State<'_, AppState>) -> Result<WorkspaceInfo> {
    trace_command("open_workspace", |_correlation_id| async move {
        state.open_workspace(std::path::Path::new(&path)).await
    })
    .await
}

#[tauri::command]
pub async fn close_workspace(state: State<'_, AppState>) -> Result<()> {
    trace_command("close_workspace", |_correlation_id| async move {
        state.close_workspace().await
    })
    .await
}

#[tauri::command]
pub async fn get_active_workspace(state: State<'_, AppState>) -> Result<Option<WorkspaceInfo>> {
    trace_command("get_active_workspace", |_correlation_id| async move {
        Ok(state.active_workspace_info().await)
    })
    .await
}

#[tauri::command]
pub async fn list_recent_workspaces(state: State<'_, AppState>) -> Result<Vec<RecentWorkspace>> {
    trace_command("list_recent_workspaces", |_correlation_id| async move {
        let workspaces = state.workspaces.read().await;
        let entries = workspaces.list_recent();
        let mut recent = Vec::with_capacity(entries.len());
        for entry in entries {
            recent.push(WorkspaceService::enrich_recent_entry(entry).await);
        }
        Ok(recent)
    })
    .await
}

#[tauri::command]
pub async fn remove_recent_workspace(path: String, state: State<'_, AppState>) -> Result<()> {
    trace_command("remove_recent_workspace", |_correlation_id| async move {
        let mut workspaces = state.workspaces.write().await;
        workspaces.remove_recent(&path)
    })
    .await
}

#[tauri::command]
pub async fn try_open_last_workspace(state: State<'_, AppState>) -> Result<Option<WorkspaceInfo>> {
    trace_command("try_open_last_workspace", |_correlation_id| async move {
        state.try_open_last_workspace().await
    })
    .await
}
