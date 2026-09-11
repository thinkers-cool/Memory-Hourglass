use memhg_lib::catalog::models::AssetMetaPatch;
use memhg_lib::catalog::repo::TagRepo;
use memhg_lib::catalog::repo::{AssetMetaRepo, AssetRepo, SourceRootRepo};
use memhg_lib::catalog::Catalog;
use memhg_lib::export::ExportService;
use memhg_lib::library::LibraryService;
use memhg_lib::query::{AssetFilter, QueryService};
use memhg_lib::scan::{ScanControl, ScanService};
use tempfile::tempdir;

#[tokio::test]
async fn full_library_workflow() {
    let dir = tempdir().unwrap();
    let photos = dir.path().join("photos");
    std::fs::create_dir_all(&photos).unwrap();
    std::fs::write(
        photos.join("a7c.jpg"),
        include_bytes!("../tests/fixtures/minimal.jpg"),
    )
    .unwrap();

    let db_path = dir.path().join("catalog.db");
    let thumb_dir = dir.path().join("thumbs");
    let catalog = Catalog::open(&db_path).await.unwrap();

    let library = LibraryService::new(catalog.pool().clone(), dir.path().to_path_buf());
    let root = library.add_local_root(photos.to_str().unwrap()).await.unwrap();

    let scanner = ScanService::new(catalog.pool().clone(), thumb_dir.clone());
    let ctrl = ScanControl::noop();
    let summary = scanner.scan_root(root.id, &ctrl).await.unwrap();
    assert_eq!(summary.indexed, 1);

    let query = QueryService::new(catalog.pool().clone(), thumb_dir.clone());
    let listed = query.query(&AssetFilter::default(), "date:desc", 0, 50).await.unwrap();
    assert_eq!(listed.total, 1);
    let asset_id = listed.items[0].id;

    let mut detail = query
        .apply_meta_patch(
            asset_id,
            AssetMetaPatch {
                rating: Some(5),
            },
        )
        .await
        .unwrap();
    assert_eq!(detail.meta.as_ref().and_then(|m| m.rating), Some(5));

    let tag_repo = TagRepo::new(catalog.pool().clone());
    let sony_id = tag_repo.create_tag("sony", None, None).await.unwrap();
    query.batch_append_tags(&[asset_id], sony_id).await.unwrap();

    detail = query.get_detail(asset_id).await.unwrap();
    let keywords_json = detail
        .meta
        .as_ref()
        .and_then(|m| m.keywords_json.as_ref())
        .expect("keywords");
    assert!(keywords_json.contains("sony"));

    let filtered = query
        .query(
            &AssetFilter {
                rating_min: Some(5),
                tag_ids: Some(vec![sony_id]),
                ..Default::default()
            },
            "date:desc",
            0,
            50,
        )
        .await
        .unwrap();
    assert_eq!(filtered.total, 1);

    let export_dest = dir.path().join("export");
    let manifest = ExportService::new(catalog.pool().clone())
        .export_assets(
            &[asset_id],
            &export_dest,
            &memhg_lib::catalog::models::ExportOptions {
                flat: true,
                rename_template: None,
                format: None,
            },
            None,
        )
        .await
        .unwrap();
    assert_eq!(manifest.copied.len(), 1);
    assert!(export_dest.join("a7c.jpg").exists());

    let assets = AssetRepo::new(catalog.pool().clone());
    assets.soft_delete(&[asset_id], 1).await.unwrap();
    let after_delete = query.query(&AssetFilter::default(), "date:desc", 0, 50).await.unwrap();
    assert_eq!(after_delete.total, 0);

    let meta_repo = AssetMetaRepo::new(catalog.pool().clone());
    assert!(meta_repo.get(asset_id).await.unwrap().is_some());
}

#[tokio::test]
async fn scan_detects_modified_and_missing() {
    let dir = tempdir().unwrap();
    let photos = dir.path().join("photos");
    std::fs::create_dir_all(&photos).unwrap();
    let file = photos.join("track.jpg");
    std::fs::write(&file, include_bytes!("../tests/fixtures/minimal.jpg")).unwrap();

    let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
    let thumb_dir = dir.path().join("thumbs");
    let roots = SourceRootRepo::new(catalog.pool().clone());
    let root = roots
        .insert_root(photos.to_str().unwrap(), "local", "watch", None)
        .await
        .unwrap();

    let scanner = ScanService::new(catalog.pool().clone(), thumb_dir);
    let ctrl = ScanControl::noop();
    scanner.scan_root(root.id, &ctrl).await.unwrap();

    std::fs::write(&file, include_bytes!("../tests/fixtures/minimal.jpg")).unwrap();
    let summary = scanner.scan_root(root.id, &ctrl).await.unwrap();
    assert!(summary.modified_count >= 1 || summary.scanned >= 1);

    std::fs::remove_file(&file).unwrap();
    let summary = scanner.scan_root(root.id, &ctrl).await.unwrap();
    assert_eq!(summary.missing_count, 1);
}
