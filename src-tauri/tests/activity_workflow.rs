use memhg_lib::activity::record::record_metadata_changed;
use memhg_lib::activity::recorder::ActivityRecorder;
use memhg_lib::activity::revert::{undo_activity, UndoContext};
use memhg_lib::catalog::models::AssetMetaPatch;
use memhg_lib::catalog::repo::{AssetMetaRepo, AssetRepo};
use memhg_lib::catalog::Catalog;
use memhg_lib::library::LibraryService;
use memhg_lib::query::{AssetFilter, QueryService};
use memhg_lib::scan::{ScanControl, ScanService};
use tempfile::tempdir;

#[tokio::test]
async fn activity_log_records_metadata_and_undo_restores_rating() {
    let dir = tempdir().unwrap();
    let photos = dir.path().join("photos");
    std::fs::create_dir_all(&photos).unwrap();
    let jpeg = include_bytes!("fixtures/minimal.jpg");
    std::fs::write(photos.join("rated.jpg"), jpeg).unwrap();

    let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
    let pool = catalog.pool().clone();
    let thumb_dir = dir.path().join("thumbs");

    let library = LibraryService::new(pool.clone(), dir.path().to_path_buf());
    let root = library
        .add_local_root(photos.to_str().unwrap())
        .await
        .unwrap();
    ScanService::new(pool.clone(), thumb_dir.clone())
        .scan_root(root.id, &ScanControl::noop())
        .await
        .unwrap();

    let query = QueryService::new(pool.clone(), thumb_dir.clone());
    let listed = query
        .query(&AssetFilter::default(), "date:desc", 0, 10)
        .await
        .unwrap();
    let asset_id = listed.items.first().expect("asset").id;
    let asset = AssetRepo::new(pool.clone())
        .get_asset(asset_id)
        .await
        .unwrap();

    let before = AssetMetaPatch { rating: Some(3) };
    let after = AssetMetaPatch { rating: Some(4) };
    query
        .apply_meta_patch(asset_id, before.clone())
        .await
        .unwrap();
    query
        .apply_meta_patch(asset_id, after.clone())
        .await
        .unwrap();

    let recorder = ActivityRecorder::new(pool.clone());
    let activity_id = record_metadata_changed(
        &recorder,
        Some("corr-rate"),
        asset_id,
        asset.root_id,
        &asset.rel_path,
        &before,
        &after,
    )
    .await
    .unwrap();

    let entries = recorder
        .repo()
        .list_for_asset(asset_id, 0, 10)
        .await
        .unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].event_type, "asset.metadata_changed");
    assert!(entries[0].reversible);
    assert_eq!(entries[0].correlation_id.as_deref(), Some("corr-rate"));

    let undo_ctx = UndoContext {
        pool: pool.clone(),
        thumb_dir: thumb_dir.clone(),
        read_only: false,
        correlation_id: Some("corr-undo".into()),
    };
    let undo_id = undo_activity(&undo_ctx, activity_id).await.unwrap();
    assert!(undo_id > activity_id);

    let meta = AssetMetaRepo::new(pool.clone())
        .get(asset_id)
        .await
        .unwrap();
    assert_eq!(meta.and_then(|m| m.rating), Some(3));

    let original = recorder.repo().get(activity_id).await.unwrap();
    assert!(original.undone_at.is_some());
    assert!(!original.reversible);
}
