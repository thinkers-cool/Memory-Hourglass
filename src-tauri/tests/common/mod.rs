pub mod unix;

use memhg_lib::commands::export::get_export_status;
use memhg_lib::commands::library::add_root;
use memhg_lib::commands::query::query_assets;
use memhg_lib::commands::scan::{get_scan_status, start_scan, ScanProgressEvent};
use memhg_lib::query::AssetFilter;
use memhg_lib::state::AppState;
use memhg_lib::workspace;
use std::path::Path;
use std::time::Duration;
use tauri::test::{mock_builder, mock_context, noop_assets, MockRuntime};
use tauri::{App, AppHandle, Manager};
use tempfile::TempDir;

pub struct TauriFixture {
    pub app: App<MockRuntime>,
    pub hold: TempDir,
}

impl TauriFixture {
    pub async fn new() -> Self {
        let hold = tempfile::tempdir().unwrap();
        let workspace_dir = hold.path().join("Test Workspace");
        std::fs::create_dir_all(&workspace_dir).unwrap();
        workspace::init_workspace_at(&workspace_dir, false).expect("workspace");
        let state = AppState::new(hold.path().join("app-data")).expect("state");
        state
            .open_workspace(&workspace_dir)
            .await
            .expect("open workspace");
        let app = mock_builder()
            .manage(state)
            .build(mock_context(noop_assets()))
            .expect("build test app");
        Self { app, hold }
    }

    pub fn state(&self) -> tauri::State<'_, AppState> {
        self.app.state::<AppState>()
    }

    pub fn handle(&self) -> AppHandle<MockRuntime> {
        self.app.handle().clone()
    }
}

pub async fn wait_for_scan(
    state: tauri::State<'_, AppState>,
    timeout: Duration,
) -> ScanProgressEvent {
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        let status = get_scan_status(state.clone()).await.expect("scan status");
        if status.stage == "done" || status.stage == "error" {
            return status;
        }
        if tokio::time::Instant::now() >= deadline {
            panic!("scan timed out in stage {}", status.stage);
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

pub async fn wait_for_jobs_idle(state: tauri::State<'_, AppState>, timeout: Duration) {
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        let current = state
            .with_active(|ws| async move { Ok(ws.jobs.current().await) })
            .await
            .expect("jobs status");
        if current.is_none() {
            return;
        }
        if tokio::time::Instant::now() >= deadline {
            panic!("jobs timed out while running {}", current.unwrap());
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

pub async fn wait_for_export_idle(state: tauri::State<'_, AppState>, timeout: Duration) {
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        let status = get_export_status(state.clone())
            .await
            .expect("export status");
        if matches!(
            status.status.as_str(),
            "completed" | "failed" | "partial" | "cancelled"
        ) {
            return;
        }
        if tokio::time::Instant::now() >= deadline {
            panic!("export timed out with status {}", status.status);
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

pub async fn seed_scanned_asset(
    fixture: &TauriFixture,
    photos_dir: &Path,
    file_name: &str,
) -> (i64, i64) {
    memhg_lib::scan::test_hooks::reset();
    std::fs::create_dir_all(photos_dir).unwrap();
    std::fs::write(
        photos_dir.join(file_name),
        include_bytes!("../fixtures/minimal.jpg"),
    )
    .unwrap();

    let state = fixture.state();
    let handle = fixture.handle();
    let root = add_root(photos_dir.to_string_lossy().to_string(), state.clone())
        .await
        .expect("add root");

    start_scan(root.id, handle, state.clone())
        .await
        .expect("start scan");
    let status = wait_for_scan(state.clone(), Duration::from_secs(30)).await;
    assert_eq!(status.stage, "done");
    wait_for_jobs_idle(state.clone(), Duration::from_secs(30)).await;

    let listed = query_assets(
        AssetFilter::default(),
        Some("date:desc".into()),
        None,
        None,
        state.clone(),
    )
    .await
    .expect("query assets");
    let asset_id = listed
        .items
        .iter()
        .find(|item| item.file_name == file_name)
        .map(|item| item.id)
        .expect("seeded asset");
    (root.id, asset_id)
}

pub async fn seed_extra_asset(
    fixture: &TauriFixture,
    photos_dir: &Path,
    root_id: i64,
    file_name: &str,
) -> i64 {
    std::fs::write(
        photos_dir.join(file_name),
        include_bytes!("../fixtures/minimal.jpg"),
    )
    .unwrap();

    let state = fixture.state();
    let handle = fixture.handle();
    start_scan(root_id, handle, state.clone())
        .await
        .expect("start scan");
    let status = wait_for_scan(state.clone(), Duration::from_secs(30)).await;
    assert_eq!(status.stage, "done");
    wait_for_jobs_idle(state.clone(), Duration::from_secs(30)).await;

    let listed = query_assets(
        AssetFilter::default(),
        Some("date:desc".into()),
        None,
        None,
        state.clone(),
    )
    .await
    .expect("query assets");
    listed
        .items
        .iter()
        .find(|item| item.file_name == file_name)
        .map(|item| item.id)
        .expect("seeded asset")
}
