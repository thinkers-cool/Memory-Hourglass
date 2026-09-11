use crate::error::Result;
use crate::state::AppState;
use crate::workspace::WorkspaceService;
use crate::workspace::RecentWorkspace;
use crate::workspace::WorkspaceInfo;
use tauri::State;

#[tauri::command] pub async fn create_workspace(
    path: String,
    read_only: bool,
    state: State<'_, AppState>,
) -> Result<WorkspaceInfo> {
    let info = {
        let mut workspaces = state.workspaces.write().await;
        workspaces.create(std::path::Path::new(&path), read_only)?
    };
    state.open_workspace(std::path::Path::new(&info.path)).await?;
    Ok(info)
}

#[tauri::command] pub async fn open_workspace(path: String, state: State<'_, AppState>) -> Result<WorkspaceInfo> {
    state.open_workspace(std::path::Path::new(&path)).await
}

#[tauri::command] pub async fn close_workspace(state: State<'_, AppState>) -> Result<()> {
    state.close_workspace().await
}

#[tauri::command] pub async fn get_active_workspace(state: State<'_, AppState>) -> Result<Option<WorkspaceInfo>> {
    Ok(state.active_workspace_info().await)
}

#[tauri::command] pub async fn list_recent_workspaces(state: State<'_, AppState>) -> Result<Vec<RecentWorkspace>> {
    let workspaces = state.workspaces.read().await;
    let entries = workspaces.list_recent();
    let mut recent = Vec::with_capacity(entries.len());
    for entry in entries {
        recent.push(WorkspaceService::enrich_recent_entry(entry).await);
    }
    Ok(recent)
}

#[tauri::command] pub async fn remove_recent_workspace(path: String, state: State<'_, AppState>) -> Result<()> {
    let mut workspaces = state.workspaces.write().await;
    workspaces.remove_recent(&path)
}

#[tauri::command] pub async fn try_open_last_workspace(state: State<'_, AppState>) -> Result<Option<WorkspaceInfo>> {
    state.try_open_last_workspace().await
}
