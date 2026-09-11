use memhg_lib::commands::workspace::{
    close_workspace, create_workspace, get_active_workspace, list_recent_workspaces,
    open_workspace, remove_recent_workspace, try_open_last_workspace,
};
use memhg_lib::state::AppState;
use memhg_lib::workspace;
use tauri::Manager;
use tempfile::tempdir;

use crate::common::TauriFixture;

#[tokio::test]
async fn workspace_commands_roundtrip() {
    let fixture = TauriFixture::new().await;
    let state = fixture.state();

    let active = get_active_workspace(state.clone()).await.unwrap();
    assert!(active.is_some());

    let recent = list_recent_workspaces(state.clone()).await.unwrap();
    assert!(!recent.is_empty());

    let workspace_path = active.unwrap().path;
    remove_recent_workspace(workspace_path.clone(), state.clone())
        .await
        .unwrap();
    let after_remove = list_recent_workspaces(state.clone()).await.unwrap();
    assert!(after_remove.iter().all(|entry| entry.path != workspace_path));

    close_workspace(state.clone()).await.unwrap();
    assert!(get_active_workspace(state.clone()).await.unwrap().is_none());

    open_workspace(workspace_path.clone(), state.clone())
        .await
        .unwrap();
    assert!(get_active_workspace(state.clone()).await.unwrap().is_some());

    let opened_last = try_open_last_workspace(state.clone()).await.unwrap();
    assert!(opened_last.is_some());

    let new_dir = fixture.hold.path().join("brand-new");
    std::fs::create_dir_all(&new_dir).unwrap();
    let created = create_workspace(new_dir.to_string_lossy().to_string(), false, state.clone())
        .await
        .unwrap();
    assert!(created.path.contains("brand-new"));
}

#[tokio::test]
async fn create_workspace_rejects_existing_path() {
    let dir = tempdir().unwrap();
    let workspace = dir.path().join("existing");
    std::fs::create_dir_all(&workspace).unwrap();
    workspace::init_workspace_at(&workspace, false).unwrap();

    let app_data = dir.path().join("app");
    let state = AppState::new(app_data).unwrap();
    let app = tauri::test::mock_builder()
        .manage(state)
        .build(tauri::test::mock_context(tauri::test::noop_assets()))
        .unwrap();
    let state = app.state::<AppState>();
    let err = create_workspace(workspace.to_string_lossy().to_string(), false, state)
        .await
        .unwrap_err();
    assert!(!err.to_string().is_empty());
}
