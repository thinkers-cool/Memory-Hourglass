use memhg_lib::catalog::models::{AssetMetaPatch, ExportOptions};
use memhg_lib::commands::activity::{query_asset_activity, undo_activity};
use memhg_lib::commands::asset::{
    purge_delete, soft_delete_assets, update_asset_meta,
};
use memhg_lib::commands::catalog::rebuild_catalog;
use memhg_lib::commands::collection::save_smart_collection;
use memhg_lib::commands::export::start_export;
use memhg_lib::commands::library::{add_root, list_roots, remove_root};
use memhg_lib::commands::query::{count_assets, query_assets};
use memhg_lib::commands::scan::start_scan;
use memhg_lib::commands::tag::{batch_append_tags, create_tag, list_tags};
use memhg_lib::query::AssetFilter;
use std::time::Duration;

use crate::common::{
    seed_extra_asset, seed_scanned_asset, wait_for_export_idle, wait_for_jobs_idle, wait_for_scan,
    TauriFixture,
};

#[tokio::test]
async fn tag_filter_query_and_undo_metadata_change() {
    let fixture = TauriFixture::new().await;
    let state = fixture.state();
    let handle = fixture.handle();
    let photos = fixture.hold.path().join("tag-undo");
    let (_root_id, asset_id) = seed_scanned_asset(&fixture, &photos, "tagged.jpg").await;

    let tag = create_tag("e2e-tag".into(), None, None, state.clone())
        .await
        .unwrap();
    batch_append_tags(vec![asset_id], tag.id, handle.clone(), state.clone())
        .await
        .unwrap();
    wait_for_jobs_idle(state.clone(), Duration::from_secs(30)).await;

    let tagged = query_assets(
        AssetFilter {
            tag_ids: Some(vec![tag.id]),
            ..Default::default()
        },
        Some("date:desc".into()),
        None,
        None,
        state.clone(),
    )
    .await
    .unwrap();
    assert_eq!(tagged.total, 1);

    update_asset_meta(
        asset_id,
        AssetMetaPatch { rating: Some(2), ..Default::default() },
        handle.clone(),
        state.clone(),
    )
    .await
    .unwrap();
    update_asset_meta(
        asset_id,
        AssetMetaPatch { rating: Some(5), ..Default::default() },
        handle.clone(),
        state.clone(),
    )
    .await
    .unwrap();
    wait_for_jobs_idle(state.clone(), Duration::from_secs(30)).await;

    let activities = query_asset_activity(asset_id, Some(0), Some(20), state.clone())
        .await
        .unwrap();
    let metadata_activity = activities
        .iter()
        .filter(|entry| entry.event_type == "asset.metadata_changed" && entry.reversible)
        .max_by_key(|entry| entry.id)
        .expect("metadata activity");
    undo_activity(metadata_activity.id, state.clone()).await.unwrap();
    wait_for_jobs_idle(state.clone(), Duration::from_secs(30)).await;

    let detail = memhg_lib::commands::asset::get_asset(asset_id, state.clone())
        .await
        .unwrap();
    assert_eq!(detail.meta.as_ref().and_then(|meta| meta.rating), Some(2));
    assert!(list_tags(state.clone()).await.unwrap().iter().any(|t| t.id == tag.id));
}

#[tokio::test]
async fn smart_collection_filter_matches_saved_criteria() {
    let fixture = TauriFixture::new().await;
    let state = fixture.state();
    let handle = fixture.handle();
    let photos = fixture.hold.path().join("smart-filter");
    let (root_id, low_id) = seed_scanned_asset(&fixture, &photos, "low.jpg").await;
    let high_id = seed_extra_asset(&fixture, &photos, root_id, "high.jpg").await;

    update_asset_meta(
        low_id,
        AssetMetaPatch { rating: Some(2), ..Default::default() },
        handle.clone(),
        state.clone(),
    )
    .await
    .unwrap();
    update_asset_meta(
        high_id,
        AssetMetaPatch { rating: Some(5), ..Default::default() },
        handle.clone(),
        state.clone(),
    )
    .await
    .unwrap();
    wait_for_jobs_idle(state.clone(), Duration::from_secs(30)).await;

    let filter = AssetFilter {
        rating_min: Some(4),
        ..Default::default()
    };
    save_smart_collection("top-rated".into(), filter.clone(), state.clone())
        .await
        .unwrap();

    let matched = query_assets(
        filter,
        Some("rating:desc".into()),
        Some(0),
        Some(10),
        state.clone(),
    )
    .await
    .unwrap();
    assert_eq!(matched.total, 1);
    assert_eq!(matched.items[0].id, high_id);
}

#[tokio::test]
async fn soft_delete_purge_removes_asset_permanently() {
    let fixture = TauriFixture::new().await;
    let state = fixture.state();
    let handle = fixture.handle();
    let photos = fixture.hold.path().join("purge-path");
    let (_root_id, asset_id) = seed_scanned_asset(&fixture, &photos, "gone.jpg").await;

    soft_delete_assets(vec![asset_id], handle.clone(), state.clone())
        .await
        .unwrap();
    wait_for_jobs_idle(state.clone(), Duration::from_secs(30)).await;

    let trashed = count_assets(
        AssetFilter {
            deleted_only: Some(true),
            ..Default::default()
        },
        state.clone(),
    )
    .await
    .unwrap();
    assert_eq!(trashed, 1);

    let purged = purge_delete(vec![asset_id], "DELETE".into(), state.clone())
        .await
        .unwrap();
    assert_eq!(purged, 1);
    wait_for_jobs_idle(state.clone(), Duration::from_secs(30)).await;

    let active = count_assets(AssetFilter::default(), state.clone())
        .await
        .unwrap();
    assert_eq!(active, 0);
    let trashed_after = count_assets(
        AssetFilter {
            deleted_only: Some(true),
            ..Default::default()
        },
        state.clone(),
    )
    .await
    .unwrap();
    assert_eq!(trashed_after, 0);
    assert!(!photos.join("gone.jpg").exists());
}

#[tokio::test]
async fn undo_soft_delete_restores_asset_to_grid() {
    let fixture = TauriFixture::new().await;
    let state = fixture.state();
    let handle = fixture.handle();
    let photos = fixture.hold.path().join("undo-delete");
    let (_root_id, asset_id) = seed_scanned_asset(&fixture, &photos, "restore-me.jpg").await;

    soft_delete_assets(vec![asset_id], handle.clone(), state.clone())
        .await
        .unwrap();
    wait_for_jobs_idle(state.clone(), Duration::from_secs(30)).await;

    let activities = query_asset_activity(asset_id, Some(0), Some(20), state.clone())
        .await
        .unwrap();
    let delete_activity = activities
        .iter()
        .find(|entry| entry.event_type == "asset.soft_deleted" && entry.reversible)
        .expect("soft delete activity");
    undo_activity(delete_activity.id, state.clone()).await.unwrap();
    wait_for_jobs_idle(state.clone(), Duration::from_secs(30)).await;

    let active = count_assets(AssetFilter::default(), state.clone())
        .await
        .unwrap();
    assert_eq!(active, 1);
}

#[tokio::test]
async fn rebuild_catalog_command_clears_tags_and_rescans() {
    let fixture = TauriFixture::new().await;
    let state = fixture.state();
    let handle = fixture.handle();
    let photos = fixture.hold.path().join("rebuild");
    let (_root_id, asset_id) = seed_scanned_asset(&fixture, &photos, "survivor.jpg").await;
    wait_for_jobs_idle(state.clone(), Duration::from_secs(30)).await;

    let tag = create_tag("temp-tag".into(), None, None, state.clone())
        .await
        .unwrap();
    batch_append_tags(vec![asset_id], tag.id, handle.clone(), state.clone())
        .await
        .unwrap();
    wait_for_jobs_idle(state.clone(), Duration::from_secs(30)).await;
    assert!(!list_tags(state.clone()).await.unwrap().is_empty());

    rebuild_catalog(state.clone()).await.unwrap();
    wait_for_jobs_idle(state.clone(), Duration::from_secs(60)).await;

    assert!(list_tags(state.clone()).await.unwrap().is_empty());
    let listed = query_assets(
        AssetFilter::default(),
        Some("date:desc".into()),
        None,
        None,
        state.clone(),
    )
    .await
    .unwrap();
    let survivor_rows = listed
        .items
        .iter()
        .filter(|item| item.file_name == "survivor.jpg")
        .count();
    assert_eq!(survivor_rows, 1);
    assert!(
        listed.items.iter().any(|item| item.id == asset_id)
            || listed.items.iter().any(|item| item.file_name == "survivor.jpg")
    );
}

#[tokio::test]
async fn export_flat_rename_template_writes_expected_files() {
    let fixture = TauriFixture::new().await;
    let state = fixture.state();
    let handle = fixture.handle();
    let photos = fixture.hold.path().join("export-rename");
    let (_root_id, asset_id) = seed_scanned_asset(&fixture, &photos, "export.jpg").await;
    let dest = fixture.hold.path().join("renamed-out");

    start_export(
        vec![asset_id],
        dest.to_string_lossy().to_string(),
        Some(ExportOptions {
            flat: true,
            rename_template: Some("{name}_copy".into()),
            format: None,
        }),
        91,
        handle,
        state.clone(),
    )
    .await
    .unwrap();
    wait_for_export_idle(state.clone(), Duration::from_secs(30)).await;
    assert!(dest.join("export_copy.jpg").exists());
}

#[tokio::test]
async fn rescan_picks_up_new_files_on_existing_root() {
    let fixture = TauriFixture::new().await;
    let state = fixture.state();
    let photos = fixture.hold.path().join("rescan");
    let (root_id, _first_id) = seed_scanned_asset(&fixture, &photos, "first.jpg").await;
    let second_id = seed_extra_asset(&fixture, &photos, root_id, "second.jpg").await;

    let listed = query_assets(
        AssetFilter::default(),
        Some("date:desc".into()),
        None,
        None,
        state.clone(),
    )
    .await
    .unwrap();
    assert_eq!(listed.total, 2);
    assert!(listed.items.iter().any(|item| item.id == second_id));
}

#[tokio::test]
async fn remove_local_root_scopes_library_to_remaining_source() {
    let fixture = TauriFixture::new().await;
    let state = fixture.state();
    let handle = fixture.handle();
    let keep_dir = fixture.hold.path().join("keep-root");
    let drop_dir = fixture.hold.path().join("drop-root");
    let (_keep_root, keep_id) = seed_scanned_asset(&fixture, &keep_dir, "keep.jpg").await;

    std::fs::create_dir_all(&drop_dir).unwrap();
    std::fs::write(
        drop_dir.join("drop.jpg"),
        include_bytes!("../fixtures/minimal.jpg"),
    )
    .unwrap();
    let drop_root = add_root(drop_dir.to_string_lossy().to_string(), state.clone())
        .await
        .unwrap();
    start_scan(drop_root.id, handle.clone(), state.clone())
        .await
        .unwrap();
    let status = wait_for_scan(state.clone(), drop_root.id, Duration::from_secs(30)).await;
    assert_eq!(status.stage, "done");
    wait_for_jobs_idle(state.clone(), Duration::from_secs(30)).await;

    assert_eq!(count_assets(AssetFilter::default(), state.clone()).await.unwrap(), 2);

    remove_root(drop_root.id, state.clone()).await.unwrap();
    wait_for_jobs_idle(state.clone(), Duration::from_secs(30)).await;

    assert_eq!(list_roots(state.clone()).await.unwrap().len(), 1);
    let listed = query_assets(
        AssetFilter::default(),
        Some("date:desc".into()),
        None,
        None,
        state.clone(),
    )
    .await
    .unwrap();
    assert_eq!(listed.total, 1);
    assert_eq!(listed.items[0].id, keep_id);
}
