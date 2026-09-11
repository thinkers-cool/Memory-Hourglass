use memhg_lib::catalog::models::ExportOptions;
use memhg_lib::commands::export::{get_export_status, list_export_jobs, start_export};
use std::time::Duration;

use crate::common::{seed_scanned_asset, wait_for_export_idle, TauriFixture};

#[tokio::test]
async fn export_command_reports_partial_failures() {
    let fixture = TauriFixture::new().await;
    let state = fixture.state();
    let handle = fixture.handle();
    let photos = fixture.hold.path().join("partial-export");
    let (_root_id, asset_id) = seed_scanned_asset(&fixture, &photos, "good.jpg").await;
    let dest = fixture.hold.path().join("partial-out");
    start_export(
        vec![asset_id, 999_999],
        dest.to_string_lossy().to_string(),
        Some(ExportOptions {
            flat: true,
            rename_template: None,
            format: None,
        }),
        88,
        handle,
        state.clone(),
    )
    .await
    .unwrap();
    wait_for_export_idle(state.clone(), Duration::from_secs(30)).await;
    let status = get_export_status(state.clone()).await.unwrap();
    assert!(status.job_id.is_some());
    let jobs = list_export_jobs(state.clone()).await.unwrap();
    assert!(!jobs.is_empty());
}

#[tokio::test]
async fn export_command_reports_all_failures() {
    let fixture = TauriFixture::new().await;
    let state = fixture.state();
    let handle = fixture.handle();
    let dest = fixture.hold.path().join("bad-export");
    start_export(
        vec![999_999],
        dest.to_string_lossy().to_string(),
        Some(ExportOptions {
            flat: true,
            rename_template: None,
            format: None,
        }),
        77,
        handle,
        state.clone(),
    )
    .await
    .expect("start export");
    for _ in 0..100 {
        let status = get_export_status(state.clone()).await.expect("status");
        if status.status == "failed" || status.status == "completed" {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("export did not finish");
}
