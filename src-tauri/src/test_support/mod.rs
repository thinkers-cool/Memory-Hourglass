#![cfg(test)]

pub mod smb;
pub mod subprocess;
pub mod unix;

use crate::state::AppState;
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
        let (state, hold) = AppState::test_with_fresh_workspace().await.unwrap();
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
    root_id: i64,
    timeout: Duration,
) -> crate::commands::scan::ScanProgressEvent {
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        let status = crate::commands::scan::get_scan_status(Some(root_id), state.clone())
            .await
            .expect("scan status");
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
        let idle = state
            .with_active(|ws| async move { Ok(ws.jobs.is_idle().await) })
            .await
            .expect("jobs status");
        if idle {
            return;
        }
        if tokio::time::Instant::now() >= deadline {
            panic!("jobs timed out while still active");
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

pub async fn seed_scanned_asset(
    fixture: &TauriFixture,
    photos_dir: &Path,
    file_name: &str,
) -> (i64, i64) {
    crate::scan::test_hooks::reset();
    std::fs::create_dir_all(photos_dir).unwrap();
    std::fs::write(
        photos_dir.join(file_name),
        include_bytes!("../../tests/fixtures/minimal.jpg"),
    )
    .unwrap();

    let state = fixture.state();
    let handle = fixture.handle();
    let root =
        crate::commands::library::add_root(photos_dir.to_string_lossy().to_string(), state.clone())
            .await
            .expect("add root");

    crate::commands::scan::start_scan(root.id, handle, state.clone())
        .await
        .expect("start scan");
    let status = wait_for_scan(state.clone(), root.id, Duration::from_secs(30)).await;
    assert_eq!(status.stage, "done");
    wait_for_jobs_idle(state.clone(), Duration::from_secs(30)).await;

    let listed = crate::commands::query::query_assets(
        crate::query::AssetFilter::default(),
        Some("date:desc".into()),
        None,
        None,
        state.clone(),
    )
    .await
    .expect("query assets");
    let asset_id = listed
        .items
        .first()
        .map(|item| item.id)
        .expect("seed asset");
    (root.id, asset_id)
}
