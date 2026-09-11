use memhg_lib::catalog::models::{AssetMetaPatch, ExportOptions};
use memhg_lib::commands::activity::{query_asset_activity, undo_activity};
use memhg_lib::commands::asset::{
    batch_update_asset_meta, get_asset, restore_assets, soft_delete_assets, update_asset_meta,
};
use memhg_lib::commands::catalog::rebuild_catalog;
use memhg_lib::commands::collection::{
    add_album_items, create_album, delete_album, delete_smart_collection, get_album_asset_ids,
    list_albums, list_smart_collections, remove_album_items, save_smart_collection,
    set_album_items, update_album,
};
use memhg_lib::commands::export::{
    cancel_export, get_export_status, list_export_jobs, start_export,
};
use memhg_lib::commands::library::{list_folder_children, list_root_stats, list_roots};
use memhg_lib::commands::query::{count_assets, query_assets};
use memhg_lib::commands::scan::{cancel_scan, get_scan_status, pause_scan, resume_scan};
use memhg_lib::commands::tag::{
    batch_append_tags, batch_remove_tags, create_tag, delete_tag, list_tags, update_tag,
};
use memhg_lib::query::AssetFilter;
use std::time::Duration;

use crate::common::{seed_scanned_asset, wait_for_export_idle, wait_for_jobs_idle, TauriFixture};

#[tokio::test]
async fn library_scan_query_asset_tag_collection_export_commands() {
    let fixture = TauriFixture::new().await;
    let state = fixture.state();
    let handle = fixture.handle();
    let photos = fixture.hold.path().join("workspace-photos");
    let (_root_id, asset_id) = seed_scanned_asset(&fixture, &photos, "cmd.jpg").await;

    let roots = list_roots(state.clone()).await.unwrap();
    assert_eq!(roots.len(), 1);

    let nested = photos.join("nested");
    std::fs::create_dir_all(&nested).unwrap();
    let children = list_folder_children(photos.to_string_lossy().to_string(), state.clone())
        .await
        .unwrap();
    assert!(children.iter().any(|entry| entry.name == "nested"));

    let stats = list_root_stats(state.clone()).await.unwrap();
    assert_eq!(stats[0].asset_count, 1);

    let status = get_scan_status(state.clone()).await.unwrap();
    assert_eq!(status.stage, "done");

    pause_scan(state.clone()).await.unwrap();
    resume_scan(state.clone()).await.unwrap();
    cancel_scan(state.clone()).await.unwrap();

    let detail = get_asset(asset_id, state.clone()).await.unwrap();
    assert_eq!(detail.asset.id, asset_id);

    let rated = update_asset_meta(
        asset_id,
        AssetMetaPatch { rating: Some(5) },
        handle.clone(),
        state.clone(),
    )
    .await
    .unwrap();
    assert_eq!(rated.meta.as_ref().and_then(|meta| meta.rating), Some(5));

    let batch_count = batch_update_asset_meta(
        vec![asset_id],
        AssetMetaPatch { rating: Some(4) },
        handle.clone(),
        state.clone(),
    )
    .await
    .unwrap();
    assert_eq!(batch_count, 1);

    let tag = create_tag(
        "cmd-tag".into(),
        None,
        Some("#ff0000".into()),
        state.clone(),
    )
    .await
    .unwrap();
    let tags = list_tags(state.clone()).await.unwrap();
    assert!(tags.iter().any(|row| row.id == tag.id));

    let updated_tag = update_tag(
        tag.id,
        "cmd-tag-renamed".into(),
        Some("#00ff00".into()),
        state.clone(),
    )
    .await
    .unwrap();
    assert_eq!(updated_tag.name, "cmd-tag-renamed");

    let tagged = batch_append_tags(vec![asset_id], tag.id, handle.clone(), state.clone())
        .await
        .unwrap();
    assert_eq!(tagged, 1);

    let removed = batch_remove_tags(vec![asset_id], tag.id, handle.clone(), state.clone())
        .await
        .unwrap();
    assert_eq!(removed, 1);

    let filtered = query_assets(
        AssetFilter {
            rating_min: Some(4),
            ..Default::default()
        },
        Some("rating:desc".into()),
        Some(0),
        Some(10),
        state.clone(),
    )
    .await
    .unwrap();
    assert_eq!(filtered.total, 1);

    let total = count_assets(AssetFilter::default(), state.clone())
        .await
        .unwrap();
    assert_eq!(total, 1);

    let smart = save_smart_collection(
        "rated".into(),
        AssetFilter {
            rating_min: Some(4),
            ..Default::default()
        },
        state.clone(),
    )
    .await
    .unwrap();
    let smart_list = list_smart_collections(state.clone()).await.unwrap();
    assert!(smart_list.iter().any(|row| row.id == smart.id));

    let album = create_album(
        "cmd-album".into(),
        Some("date:desc".into()),
        Some("📷".into()),
        state.clone(),
    )
    .await
    .unwrap();
    set_album_items(album.id, vec![asset_id], state.clone())
        .await
        .unwrap();
    let added = add_album_items(album.id, vec![asset_id], state.clone())
        .await
        .unwrap();
    assert_eq!(added, 0);
    let album_ids = get_album_asset_ids(album.id, state.clone()).await.unwrap();
    assert_eq!(album_ids, vec![asset_id]);
    let removed_items = remove_album_items(album.id, vec![asset_id], state.clone())
        .await
        .unwrap();
    assert_eq!(removed_items, 1);

    let albums = list_albums(state.clone()).await.unwrap();
    assert!(albums.iter().any(|row| row.id == album.id));

    update_album(
        album.id,
        "cmd-album-2".into(),
        Some("🎞".into()),
        state.clone(),
    )
    .await
    .unwrap();

    let activities = query_asset_activity(asset_id, Some(0), Some(20), state.clone())
        .await
        .unwrap();
    assert!(!activities.is_empty());
    let undo_target = activities
        .iter()
        .find(|entry| entry.reversible)
        .map(|entry| entry.id)
        .expect("reversible activity");
    undo_activity(undo_target, state.clone()).await.unwrap();

    wait_for_jobs_idle(state.clone(), Duration::from_secs(30)).await;

    let export_dest = fixture.hold.path().join("export-out");
    start_export(
        vec![asset_id],
        export_dest.to_string_lossy().to_string(),
        Some(ExportOptions {
            flat: true,
            rename_template: None,
            format: None,
        }),
        42,
        handle.clone(),
        state.clone(),
    )
    .await
    .unwrap();
    wait_for_export_idle(state.clone(), Duration::from_secs(30)).await;
    let jobs = list_export_jobs(state.clone()).await.unwrap();
    assert!(!jobs.is_empty());
    let export_status = get_export_status(state.clone()).await.unwrap();
    assert!(export_status.job_id.is_some() || export_status.status != "idle");
    cancel_export(state.clone()).await.unwrap();

    delete_tag(tag.id, state.clone()).await.unwrap();
    delete_smart_collection(smart.id, state.clone())
        .await
        .unwrap();
    delete_album(album.id, state.clone()).await.unwrap();

    let deleted = soft_delete_assets(vec![asset_id], handle.clone(), state.clone())
        .await
        .unwrap();
    assert_eq!(deleted, 1);
    let restored = restore_assets(vec![asset_id], state.clone()).await.unwrap();
    assert_eq!(restored, 1);

    rebuild_catalog(state.clone()).await.unwrap();
    crate::common::wait_for_jobs_idle(state.clone(), std::time::Duration::from_secs(30)).await;
}
