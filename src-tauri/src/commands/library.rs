use crate::catalog::models::SourceRoot;
use crate::commands::context::trace_command;
use crate::error::Result;
use crate::library::{FolderEntry, RelinkPreview, RootStats, SmbSourceInput};
use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub async fn add_root(path: String, state: State<'_, AppState>) -> Result<SourceRoot> {
    trace_command("add_root", || async move {
        state
            .with_active(|ws| async move {
                let root = ws.library.add_local_root(&path).await?;
                ws.notify_roots_refresh();
                Ok(root)
            })
            .await
    })
    .await
}

#[tauri::command]
pub async fn add_smb_source(
    input: SmbSourceInput,
    state: State<'_, AppState>,
) -> Result<SourceRoot> {
    trace_command("add_smb_source", || async move {
        state
            .with_active(|ws| async move {
                let root = ws.library.add_smb_source(input).await?;
                ws.notify_roots_refresh();
                Ok(root)
            })
            .await
    })
    .await
}

#[tauri::command]
pub async fn relink_root(id: i64, path: String, state: State<'_, AppState>) -> Result<SourceRoot> {
    trace_command("relink_root", || async move {
        state
            .with_active(|ws| async move { ws.library.relink_root(id, &path).await })
            .await
    })
    .await
}

#[tauri::command]
pub async fn preview_relink(
    id: i64,
    path: String,
    state: State<'_, AppState>,
) -> Result<RelinkPreview> {
    trace_command("preview_relink", || async move {
        state
            .with_active(|ws| async move { ws.library.preview_relink(id, &path).await })
            .await
    })
    .await
}

#[tauri::command]
pub async fn list_root_stats(state: State<'_, AppState>) -> Result<Vec<RootStats>> {
    trace_command("list_root_stats", || async move {
        state
            .with_active(|ws| async move { ws.library.list_root_stats().await })
            .await
    })
    .await
}

#[tauri::command]
pub async fn remove_root(id: i64, state: State<'_, AppState>) -> Result<()> {
    trace_command("remove_root", || async move {
        state
            .with_active(|ws| async move {
                ws.terminate_root_jobs(id).await;
                ws.library.remove_root(id).await?;
                ws.notify_roots_refresh();
                Ok(())
            })
            .await
    })
    .await
}

#[tauri::command]
pub async fn list_roots(state: State<'_, AppState>) -> Result<Vec<SourceRoot>> {
    trace_command("list_roots", || async move {
        state
            .with_active(|ws| async move { ws.library.list_roots().await })
            .await
    })
    .await
}

#[tauri::command]
pub async fn list_folder_children(
    path: String,
    state: State<'_, AppState>,
) -> Result<Vec<FolderEntry>> {
    trace_command("list_folder_children", || async move {
        state
            .with_active(|ws| async move { ws.library.list_folder_children(&path).await })
            .await
    })
    .await
}

#[cfg(test)]
mod tests {
    use crate::state::AppState;
    use tempfile::tempdir;

    #[tokio::test]
    async fn add_root_notifies_refresh() {
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        let (state, _guard) = AppState::test_with_fresh_workspace().await.unwrap();
        state
            .with_active(|ws| async move {
                let mut refresh = ws.roots_refresh.subscribe();
                let root = ws
                    .library
                    .add_local_root(photos.to_str().unwrap())
                    .await
                    .unwrap();
                ws.notify_roots_refresh();
                refresh.changed().await.unwrap();
                assert!(root.id > 0);
                assert_eq!(*refresh.borrow(), 1);
                Ok(())
            })
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn list_roots_starts_empty() {
        let (state, _guard) = AppState::test_with_fresh_workspace().await.unwrap();
        let roots = state
            .with_active(|ws| async move { ws.library.list_roots().await })
            .await
            .unwrap();
        assert!(roots.is_empty());
    }

    #[tokio::test]
    async fn remove_root_terminates_pending_scan_jobs() {
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        let (state, _guard) = AppState::test_with_fresh_workspace().await.unwrap();
        state
            .with_active(|ws| async move {
                let root = ws
                    .library
                    .add_local_root(photos.to_str().unwrap())
                    .await
                    .unwrap();
                ws.jobs.enqueue_scan(root.id).await;
                ws.terminate_root_jobs(root.id).await;
                assert!(ws.jobs.pending_scan_roots().await.is_empty());
                assert!(!ws.jobs.is_scan_running(root.id).await);
                assert!(!ws.scan_status.read().await.contains_key(&root.id));
                ws.library.remove_root(root.id).await.unwrap();
                Ok(())
            })
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn remove_root_notifies_refresh() {
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        let (state, _guard) = AppState::test_with_fresh_workspace().await.unwrap();
        state
            .with_active(|ws| async move {
                let root = ws
                    .library
                    .add_local_root(photos.to_str().unwrap())
                    .await
                    .unwrap();
                let mut refresh = ws.roots_refresh.subscribe();
                ws.library.remove_root(root.id).await.unwrap();
                ws.notify_roots_refresh();
                refresh.changed().await.unwrap();
                assert!(ws.library.list_roots().await.unwrap().is_empty());
                Ok(())
            })
            .await
            .unwrap();
    }
}
