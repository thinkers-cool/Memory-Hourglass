use memhg_lib::commands::activity::{query_asset_activity, undo_activity};
use memhg_lib::commands::workspace::{close_workspace, open_workspace};

use crate::common::TauriFixture;

#[tokio::test]
async fn activity_command_reports_query_errors() {
    let fixture = TauriFixture::new().await;
    let state = fixture.state();
    close_workspace(state.clone()).await.unwrap();
    let err = query_asset_activity(1, None, None, state.clone())
        .await
        .unwrap_err();
    assert!(err.to_string().contains("no workspace open"));

    open_workspace(
        fixture
            .hold
            .path()
            .join("Test Workspace")
            .to_string_lossy()
            .to_string(),
        state.clone(),
    )
    .await
    .unwrap();
    let undo_err = undo_activity(999_999, state.clone()).await.unwrap_err();
    assert!(!undo_err.to_string().is_empty());
}
