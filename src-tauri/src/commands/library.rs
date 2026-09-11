use crate::catalog::models::SourceRoot;
use crate::error::Result;
use crate::library::{FolderEntry, RelinkPreview, RootStats, SmbSourceInput};
use crate::state::AppState;
use tauri::State;

#[tauri::command] pub async fn add_root(path: String, state: State<'_, AppState>) -> Result<SourceRoot> {
    state
        .with_active(|ws| async move {
            let root = ws.library.add_local_root(&path).await?;
            ws.notify_roots_refresh();
            Ok(root)
        })
        .await
}

#[tauri::command] pub async fn add_smb_source(
    input: SmbSourceInput,
    state: State<'_, AppState>,
) -> Result<SourceRoot> {
    state
        .with_active(|ws| async move {
            let root = ws.library.add_smb_source(input).await?;
            ws.notify_roots_refresh();
            Ok(root)
        })
        .await
}

#[tauri::command] pub async fn relink_root(
    id: i64,
    path: String,
    state: State<'_, AppState>,
) -> Result<SourceRoot> {
    state
        .with_active(|ws| async move { ws.library.relink_root(id, &path).await })
        .await
}

#[tauri::command] pub async fn preview_relink(
    id: i64,
    path: String,
    state: State<'_, AppState>,
) -> Result<RelinkPreview> {
    state
        .with_active(|ws| async move { ws.library.preview_relink(id, &path).await })
        .await
}

#[tauri::command] pub async fn list_root_stats(state: State<'_, AppState>) -> Result<Vec<RootStats>> {
    state
        .with_active(|ws| async move { ws.library.list_root_stats().await })
        .await
}

#[tauri::command] pub async fn remove_root(id: i64, state: State<'_, AppState>) -> Result<()> {
    state
        .with_active(|ws| async move {
            ws.library.remove_root(id).await?;
            ws.notify_roots_refresh();
            Ok(())
        })
        .await
}

#[tauri::command] pub async fn list_roots(state: State<'_, AppState>) -> Result<Vec<SourceRoot>> {
    state
        .with_active(|ws| async move { ws.library.list_roots().await })
        .await
}

#[tauri::command] pub async fn list_folder_children(
    path: String,
    state: State<'_, AppState>,
) -> Result<Vec<FolderEntry>> {
    state
        .with_active(|ws| async move { ws.library.list_folder_children(&path).await })
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
                let root = ws.library.add_local_root(photos.to_str().unwrap()).await.unwrap();
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
    async fn remove_root_notifies_refresh() {
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        let (state, _guard) = AppState::test_with_fresh_workspace().await.unwrap();
        state
            .with_active(|ws| async move {
                let root = ws.library.add_local_root(photos.to_str().unwrap()).await.unwrap();
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
