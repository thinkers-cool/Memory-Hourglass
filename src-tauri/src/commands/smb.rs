use crate::commands::context::trace_command;
use crate::error::Result;
use crate::smb::{list_shares, SmbConnectRequest, SmbListRequest, SmbShareEntry};
use tauri::State;

use crate::state::AppState;

#[tauri::command]
pub async fn list_smb_shares(
    request: SmbListRequest,
    _state: State<'_, AppState>,
) -> Result<Vec<SmbShareEntry>> {
    trace_command("list_smb_shares", |_correlation_id| async move {
        tokio::task::spawn_blocking(move || list_shares(&request))
            .await
            .map_err(|error| {
                crate::error::AppError::Library(format!("SMB listing failed: {}", error))
            })?
    })
    .await
}

#[tauri::command]
pub async fn mount_smb_for_browse(
    request: SmbConnectRequest,
    state: State<'_, AppState>,
) -> Result<String> {
    trace_command("mount_smb_for_browse", |_correlation_id| async move {
        state
            .with_active(|ws| async move { ws.library.mount_smb_for_browse(&request).await })
            .await
    })
    .await
}
