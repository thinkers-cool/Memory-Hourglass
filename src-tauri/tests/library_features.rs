use memhg_lib::catalog::models::{AssetMetaPatch, ExportOptions};
use memhg_lib::catalog::repo::{AssetRepo, SourceRootRepo};
use memhg_lib::catalog::Catalog;
use memhg_lib::collection::CollectionRepo;
use memhg_lib::export::ExportService;
use memhg_lib::library::{LibraryService, SmbSourceInput};
use memhg_lib::link::LinkService;
use memhg_lib::query::{AssetFilter, QueryService};
use memhg_lib::scan::{ScanControl, ScanService};
use memhg_lib::workspace::WorkspacePaths;
use tempfile::tempdir;

#[tokio::test]
async fn albums_smart_collections_and_raw_link() {
    let dir = tempdir().unwrap();
    let photos = dir.path().join("photos");
    std::fs::create_dir_all(&photos).unwrap();
    let jpeg = include_bytes!("../tests/fixtures/minimal.jpg");
    std::fs::write(photos.join("DSC100.jpg"), jpeg).unwrap();
    std::fs::write(photos.join("DSC100.arw"), jpeg).unwrap();

    let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
    let pool = catalog.pool().clone();
    let thumb_dir = dir.path().join("thumbs");

    let library = LibraryService::new(pool.clone(), dir.path().to_path_buf());
    let root = library
        .add_local_root(photos.to_str().unwrap())
        .await
        .unwrap();
    let smb_photos = dir.path().join("smb_photos");
    std::fs::create_dir_all(&smb_photos).unwrap();
    std::fs::write(smb_photos.join("net.jpg"), jpeg).unwrap();
    let smb = library
        .add_smb_source(SmbSourceInput::Mounted {
            path: smb_photos.to_string_lossy().to_string(),
            poll_secs: Some(120),
        })
        .await
        .unwrap();
    assert_eq!(smb.kind, "smb");

    let ctrl = ScanControl::noop();
    ScanService::new(pool.clone(), thumb_dir.clone())
        .scan_root(root.id, &ctrl)
        .await
        .unwrap();

    let link = LinkService::new(pool.clone());
    let linked = link.link_raw_jpeg_in_root(root.id).await.unwrap();
    assert_eq!(linked, 1);

    let query = QueryService::new(pool.clone(), thumb_dir.clone());
    let assets = query
        .query(&AssetFilter::default(), "date:desc", 0, 50)
        .await
        .unwrap();
    assert_eq!(assets.total, 2);

    let collection = CollectionRepo::new(pool.clone());
    let smart = collection
        .save_smart_collection(
            "high rated",
            &AssetFilter {
                kind: Some("raw".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert!(smart.id > 0);

    let album = collection
        .create_album("trip", "date:desc", None)
        .await
        .unwrap();
    let ids = assets.items.iter().map(|a| a.id).collect::<Vec<_>>();
    collection.set_album_items(album.id, &ids).await.unwrap();
    let album_ids = collection.album_asset_ids(album.id).await.unwrap();
    assert_eq!(album_ids.len(), 2);

    let stats = library.list_root_stats().await.unwrap();
    assert_eq!(stats.len(), 2);
    assert!(stats.iter().any(|s| s.asset_count == 2));
}

#[tokio::test]
async fn duplicates_batch_meta_export_and_relink() {
    let dir = tempdir().unwrap();
    let photos = dir.path().join("photos");
    let moved = dir.path().join("moved");
    std::fs::create_dir_all(&photos).unwrap();
    let jpeg = include_bytes!("../tests/fixtures/minimal.jpg");
    std::fs::write(photos.join("dup.jpg"), jpeg).unwrap();
    std::fs::write(photos.join("copy.jpg"), jpeg).unwrap();

    let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
    let pool = catalog.pool().clone();
    let thumb_dir = dir.path().join("thumbs");

    let library = LibraryService::new(pool.clone(), dir.path().to_path_buf());
    let root = library
        .add_local_root(photos.to_str().unwrap())
        .await
        .unwrap();
    let ctrl = ScanControl::noop();
    ScanService::new(pool.clone(), thumb_dir.clone())
        .scan_root(root.id, &ctrl)
        .await
        .unwrap();

    let link = LinkService::new(pool.clone());
    let hashed = link
        .compute_hashes_for_root(root.id, &photos)
        .await
        .unwrap();
    assert_eq!(hashed, 0);
    let dups = link.find_duplicates_by_hash(Some(root.id)).await.unwrap();
    assert_eq!(dups.len(), 1);
    assert_eq!(dups[0].len(), 2);

    let query = QueryService::new(pool.clone(), thumb_dir.clone());
    let listed = query
        .query(&AssetFilter::default(), "date:desc", 0, 50)
        .await
        .unwrap();
    let ids = listed.items.iter().map(|a| a.id).collect::<Vec<_>>();

    let updated = query
        .batch_apply_meta(&ids, AssetMetaPatch { rating: Some(3) })
        .await
        .unwrap();
    assert_eq!(updated, 2);

    let export_dest = dir.path().join("export");
    let manifest = ExportService::new(pool.clone())
        .export_assets(
            &ids,
            &export_dest,
            &ExportOptions {
                flat: true,
                rename_template: Some("{name}_export".into()),
                format: None,
            },
            None,
        )
        .await
        .unwrap();
    assert_eq!(manifest.copied.len(), 2);

    std::fs::create_dir_all(&moved).unwrap();
    std::fs::copy(photos.join("dup.jpg"), moved.join("dup.jpg")).unwrap();
    std::fs::copy(photos.join("copy.jpg"), moved.join("copy.jpg")).unwrap();

    let preview = library
        .preview_relink(root.id, moved.to_str().unwrap())
        .await
        .unwrap();
    assert_eq!(preview.matched, preview.total_sampled);

    library
        .relink_root(root.id, moved.to_str().unwrap())
        .await
        .unwrap();
    ScanService::new(pool.clone(), thumb_dir.clone())
        .scan_root(root.id, &ctrl)
        .await
        .unwrap();

    let after = query
        .query(&AssetFilter::default(), "date:desc", 0, 50)
        .await
        .unwrap();
    assert_eq!(after.total, 2);

    let assets = AssetRepo::new(pool.clone());
    let purge_id = ids[0];
    let paths = WorkspacePaths::new(dir.path().to_path_buf());
    assets
        .purge_assets(&[purge_id], &paths, false)
        .await
        .unwrap();
    let remaining = query
        .query(&AssetFilter::default(), "date:desc", 0, 50)
        .await
        .unwrap();
    assert_eq!(remaining.total, 1);
}

#[tokio::test]
async fn offline_root_returns_without_error() {
    let dir = tempdir().unwrap();
    let photos = dir.path().join("photos");
    std::fs::create_dir_all(&photos).unwrap();
    std::fs::write(
        photos.join("one.jpg"),
        include_bytes!("../tests/fixtures/minimal.jpg"),
    )
    .unwrap();

    let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
    let pool = catalog.pool().clone();
    let thumb_dir = dir.path().join("thumbs");
    let roots = SourceRootRepo::new(pool.clone());
    let root = roots
        .insert_root(photos.to_str().unwrap(), "smb", "poll", Some(300))
        .await
        .unwrap();

    let ctrl = ScanControl::noop();
    ScanService::new(pool.clone(), thumb_dir)
        .scan_root(root.id, &ctrl)
        .await
        .unwrap();

    std::fs::remove_dir_all(&photos).unwrap();
    let summary = ScanService::new(pool.clone(), dir.path().join("thumbs2"))
        .scan_root(root.id, &ctrl)
        .await
        .unwrap();
    assert_eq!(summary.indexed, 0);

    let root_row = roots.get_root(root.id).await.unwrap();
    assert_eq!(root_row.status, "offline");
}
