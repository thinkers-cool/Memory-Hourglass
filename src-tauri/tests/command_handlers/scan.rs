use memhg_lib::commands::scan::start_scan;
use std::time::Duration;

use crate::common::{wait_for_scan, TauriFixture};

#[tokio::test]
async fn scan_command_handles_missing_root() {
    memhg_lib::scan::test_hooks::reset();
    let fixture = TauriFixture::new().await;
    let state = fixture.state();
    let handle = fixture.handle();

    start_scan(999_999, handle, state.clone()).await.unwrap();
    let status = wait_for_scan(state.clone(), Duration::from_secs(10)).await;
    assert_eq!(status.stage, "error");
}
