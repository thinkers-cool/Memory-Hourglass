use crate::error::Result;
use crate::smb::{list_shares, SmbConnectRequest, SmbListRequest, SmbShareEntry};
use tauri::State;

use crate::state::AppState;

#[tauri::command] pub async fn list_smb_shares(
    request: SmbListRequest,
    _state: State<'_, AppState>,
) -> Result<Vec<SmbShareEntry>> {
    tokio::task::spawn_blocking(move || list_shares(&request))
        .await
        .map_err(|error| crate::error::AppError::Library(format!("SMB listing failed: {}", error)))?
}

#[tauri::command] pub async fn mount_smb_for_browse(
    request: SmbConnectRequest,
    state: State<'_, AppState>,
) -> Result<String> {
    state
        .with_active(|ws| async move { ws.library.mount_smb_for_browse(&request).await })
        .await
}
