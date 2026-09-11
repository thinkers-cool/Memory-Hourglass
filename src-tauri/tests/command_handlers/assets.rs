use memhg_lib::commands::asset::{purge_delete, restore_assets, soft_delete_assets};
use memhg_lib::commands::library::list_roots;
use memhg_lib::commands::query::query_assets;
use memhg_lib::query::AssetFilter;
use memhg_lib::state::AppState;
use memhg_lib::workspace;
use std::time::Duration;
use tauri::Manager;

use crate::common::{seed_scanned_asset, wait_for_jobs_idle, TauriFixture};

#[tokio::test]
async fn asset_and_query_validation_errors() {
    let fixture = TauriFixture::new().await;
    let state = fixture.state();
    let handle = fixture.handle();

    let purge_err = purge_delete(vec![1], "NOPE".into(), state.clone())
        .await
        .unwrap_err();
    assert!(purge_err.to_string().contains("confirm_token"));

    let query_err = query_assets(
        AssetFilter::default(),
        None,
        None,
        None,
        state.clone(),
    )
    .await
    .unwrap_err();
    assert!(query_err.to_string().contains("sort is required"));

    let no_workspace = AppState::new(fixture.hold.path().join("other-app-data"))
        .unwrap();
    let app = tauri::test::mock_builder()
        .manage(no_workspace)
        .build(tauri::test::mock_context(tauri::test::noop_assets()))
        .unwrap();
    let empty_state = app.state::<AppState>();
    let closed_err = list_roots(empty_state).await.unwrap_err();
    assert!(closed_err.to_string().contains("no workspace open"));

    let photos = fixture.hold.path().join("purge-photos");
    let (_root_id, asset_id) = seed_scanned_asset(&fixture, &photos, "purge.jpg").await;
    wait_for_jobs_idle(state.clone(), Duration::from_secs(30)).await;
    let purged = purge_delete(vec![asset_id], "DELETE".into(), state.clone())
        .await
        .expect("purge asset");
    assert_eq!(purged, 1);

    let zero_delete = soft_delete_assets(vec![asset_id], handle, state.clone())
        .await
        .expect("soft delete purged asset");
    assert_eq!(zero_delete, 0);
}

#[tokio::test]
async fn asset_commands_handle_zero_counts_and_read_only_purge() {
    let fixture = TauriFixture::new().await;
    let state = fixture.state();
    let handle = fixture.handle();

    let photos = fixture.hold.path().join("purge-assets");
    let (_root_id, asset_id) = seed_scanned_asset(&fixture, &photos, "one.jpg").await;

    let zero = soft_delete_assets(
        vec![asset_id + 999],
        handle.clone(),
        state.clone(),
    )
    .await
    .expect("soft delete");
    assert_eq!(zero, 0);

    let read_only_dir = fixture.hold.path().join("Read Only");
    std::fs::create_dir_all(&read_only_dir).unwrap();
    workspace::init_workspace_at(&read_only_dir, true).expect("read only ws");
    state
        .open_workspace(&read_only_dir)
        .await
        .expect("open read only");
    let err = purge_delete(vec![asset_id], "DELETE".into(), state.clone())
        .await
        .expect_err("purge blocked");
    assert!(err.to_string().contains("read-only"));
}

#[tokio::test]
async fn restore_and_purge_commands_record_activity() {
    let fixture = TauriFixture::new().await;
    let state = fixture.state();
    let handle = fixture.handle();

    let photos = fixture.hold.path().join("activity-assets");
    let (_root_id, asset_id) = seed_scanned_asset(&fixture, &photos, "purge-me.jpg").await;

    soft_delete_assets(vec![asset_id], handle.clone(), state.clone())
        .await
        .expect("soft delete");
    let restored = restore_assets(vec![asset_id], state.clone())
        .await
        .expect("restore");
    assert_eq!(restored, 1);

    soft_delete_assets(vec![asset_id], handle.clone(), state.clone())
        .await
        .expect("soft delete again");
    restore_assets(vec![asset_id], state.clone())
        .await
        .expect("restore again");
    let purged = purge_delete(vec![asset_id], "DELETE".into(), state.clone())
        .await
        .expect("purge");
    assert_eq!(purged, 1);
}
