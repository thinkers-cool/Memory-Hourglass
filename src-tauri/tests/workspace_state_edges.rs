mod common;

use memhg_lib::state::{ActiveWorkspace, AppState};
use memhg_lib::workspace::{self, WorkspaceService, WORKSPACE_MANIFEST};
use std::path::Path;
use tempfile::tempdir;

use crate::common::unix::{allow_access, deny_access, write_executable};

#[tokio::test]
async fn active_workspace_open_rejects_missing_path() {
    let dir = tempdir().unwrap();
    let result = ActiveWorkspace::open(
        &dir.path().join("missing-workspace"),
        &dir.path().join("app"),
    )
    .await;
    assert!(result.is_err());
    assert!(result.err().unwrap().to_string().contains("not found"));
}

#[tokio::test]
async fn active_workspace_open_rejects_corrupt_catalog() {
    let dir = tempdir().unwrap();
    let ws_path = dir.path().join("Broken");
    std::fs::create_dir_all(&ws_path).unwrap();
    workspace::init_workspace_at(&ws_path, false).unwrap();
    std::fs::write(ws_path.join("catalog.db"), b"not-a-database").unwrap();
    assert!(ActiveWorkspace::open(&ws_path, &dir.path().join("app"))
        .await
        .is_err());
}

#[tokio::test]
async fn active_workspace_open_rejects_invalid_manifest() {
    let dir = tempdir().unwrap();
    let ws_path = dir.path().join("BadManifest");
    std::fs::create_dir_all(&ws_path).unwrap();
    workspace::init_workspace_at(&ws_path, false).unwrap();
    std::fs::write(ws_path.join(WORKSPACE_MANIFEST), b"{bad-json").unwrap();
    let result = ActiveWorkspace::open(&ws_path, &dir.path().join("app")).await;
    assert!(result.is_err());
    assert!(result.err().unwrap().to_string().contains("workspace.json"));
}

#[cfg(unix)]
#[tokio::test]
async fn active_workspace_open_rejects_unwritable_mount_parent() {
    let dir = tempdir().unwrap();
    let ws_path = dir.path().join("MountFail");
    std::fs::create_dir_all(&ws_path).unwrap();
    workspace::init_workspace_at(&ws_path, false).unwrap();
    let app_data = dir.path().join("app-data-file");
    std::fs::write(&app_data, b"blocker").unwrap();
    deny_access(&app_data);
    let result = ActiveWorkspace::open(&ws_path, &app_data).await;
    allow_access(&app_data, 0o644);
    assert!(result.is_err());
}

#[cfg(unix)]
#[test]
fn app_state_new_rejects_file_app_data_path() {
    let dir = tempdir().unwrap();
    let blocker = dir.path().join("app-data");
    std::fs::write(&blocker, b"not-a-directory").unwrap();
    assert!(AppState::new(blocker).is_err());
}

#[tokio::test]
async fn try_open_last_workspace_returns_error_when_open_fails() {
    let dir = tempdir().unwrap();
    let ws_path = dir.path().join("RecentBroken");
    std::fs::create_dir_all(&ws_path).unwrap();
    let info = workspace::init_workspace_at(&ws_path, false).unwrap();
    let app_data = dir.path().join("app");
    let mut service = WorkspaceService::load(app_data.clone()).unwrap();
    service.touch_opened(&info).unwrap();
    std::fs::write(ws_path.join("catalog.db"), b"broken").unwrap();
    let state = AppState::new(app_data).unwrap();
    assert!(state.try_open_last_workspace().await.is_err());
}

#[cfg(unix)]
#[tokio::test]
async fn open_workspace_fails_when_registry_unwritable() {
    let dir = tempdir().unwrap();
    let ws_path = dir.path().join("RegistryFail");
    std::fs::create_dir_all(&ws_path).unwrap();
    let info = workspace::init_workspace_at(&ws_path, false).unwrap();
    let app_data = dir.path().join("app");
    let state = AppState::new(app_data.clone()).unwrap();
    state.open_workspace(Path::new(&info.path)).await.unwrap();
    let second = dir.path().join("Second");
    std::fs::create_dir_all(&second).unwrap();
    let second_info = workspace::init_workspace_at(&second, false).unwrap();
    let registry = app_data.join("workspaces.json");
    deny_access(&registry);
    let result = state.open_workspace(Path::new(&second_info.path)).await;
    allow_access(&registry, 0o644);
    assert!(result.is_err());
}

#[cfg(unix)]
#[test]
fn workspace_registry_save_fails_when_app_data_unwritable() {
    let dir = tempdir().unwrap();
    let app_data = dir.path().join("app");
    let mut service = WorkspaceService::load(app_data.clone()).unwrap();
    service
        .registry_mut()
        .touch_recent(&memhg_lib::workspace::WorkspaceInfo {
            path: "/tmp/ws".into(),
            name: "Demo".into(),
            id: "id".into(),
            read_only: false,
        });
    service.save_registry().unwrap();
    let registry = app_data.join("workspaces.json");
    deny_access(&registry);
    assert!(service.save_registry().is_err());
    allow_access(&registry, 0o644);
}

#[cfg(unix)]
#[test]
fn workspace_registry_load_fails_when_registry_unreadable() {
    let dir = tempdir().unwrap();
    let app_data = dir.path().join("app");
    std::fs::create_dir_all(&app_data).unwrap();
    let registry = app_data.join("workspaces.json");
    std::fs::write(&registry, r#"{"recent":[],"last_opened":null}"#).unwrap();
    deny_access(&registry);
    assert!(WorkspaceService::load(app_data).is_err());
    allow_access(&registry, 0o644);
}

#[cfg(unix)]
#[test]
fn init_workspace_propagates_read_dir_failure_on_locked_directory() {
    let dir = tempdir().unwrap();
    let locked = dir.path().join("locked");
    std::fs::create_dir_all(&locked).unwrap();
    deny_access(&locked);
    let result = workspace::init_workspace_at(&locked, false);
    allow_access(&locked, 0o755);
    assert!(result.is_err());
}

#[cfg(target_os = "macos")]
#[tokio::test]
async fn connect_smb_share_reports_mount_failure() {
    use memhg_lib::catalog::Catalog;
    use memhg_lib::library::LibraryService;
    use memhg_lib::smb::SmbConnectRequest;

    let dir = tempdir().unwrap();
    let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
    let mount_dir = dir.path().join("mounts");
    std::fs::create_dir_all(&mount_dir).unwrap();
    let script = mount_dir.join("mount_smbfs.sh");
    write_executable(&script, "#!/bin/sh\nexit 1\n");
    std::env::remove_var("MEMHG_TEST_MOUNT_SMBFS");
    std::env::set_var("MEMHG_TEST_MOUNT_SMBFS", script.to_string_lossy().as_ref());
    let library = LibraryService::new(catalog.pool().clone(), mount_dir);
    let result = library
        .connect_smb_share(&SmbConnectRequest {
            host: "nas".into(),
            share: "photos".into(),
            username: "guest".into(),
            password: "secret".into(),
            domain: None,
            poll_secs: None,
            sub_path: None,
            read_only: false,
        })
        .await;
    std::env::remove_var("MEMHG_TEST_MOUNT_SMBFS");
    assert!(result.is_err());
}
