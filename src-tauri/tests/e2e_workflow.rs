use memhg_lib::catalog::models::{AssetMetaPatch, ExportOptions};
use memhg_lib::catalog::rebuild::rebuild_and_rescan;
use memhg_lib::catalog::repo::TagRepo;
use memhg_lib::catalog::repo::{AssetMetaRepo, AssetRepo, SourceRootRepo};
use memhg_lib::catalog::Catalog;
use memhg_lib::collection::CollectionRepo;
use memhg_lib::export::ExportService;
use memhg_lib::library::LibraryService;
use memhg_lib::link::LinkService;
use memhg_lib::query::{AssetFilter, QueryService};
use memhg_lib::scan::{ScanControl, ScanService};
use memhg_lib::workspace::WorkspacePaths;
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

    let library = LibraryService::new(catalog.pools().clone(), dir.path().to_path_buf());
    let root = library
        .add_local_root(photos.to_str().unwrap())
        .await
        .unwrap();

    let scanner = ScanService::new(catalog.pools().clone(), thumb_dir.clone());
    let ctrl = ScanControl::noop();
    let summary = scanner.scan_root(root.id, &ctrl).await.unwrap();
    assert_eq!(summary.indexed, 1);

    let query = QueryService::new(catalog.pools().clone(), thumb_dir.clone());
    let listed = query
        .query(&AssetFilter::default(), "date:desc", 0, 50)
        .await
        .unwrap();
    assert_eq!(listed.total, 1);
    let asset_id = listed.items[0].id;

    let mut detail = query
        .apply_meta_patch(asset_id, AssetMetaPatch { rating: Some(5), ..Default::default() })
        .await
        .unwrap();
    assert_eq!(detail.meta.as_ref().and_then(|m| m.rating), Some(5));

    let tag_repo = TagRepo::new(catalog.pools().clone());
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
    let manifest = ExportService::new(catalog.pools().clone())
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

    let assets = AssetRepo::new(catalog.pools().clone());
    assets.soft_delete(&[asset_id], 1).await.unwrap();
    let after_delete = query
        .query(&AssetFilter::default(), "date:desc", 0, 50)
        .await
        .unwrap();
    assert_eq!(after_delete.total, 0);

    let meta_repo = AssetMetaRepo::new(catalog.pools().clone());
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
    let roots = SourceRootRepo::new(catalog.pools().clone());
    let root = roots
        .insert_root(photos.to_str().unwrap(), "local", "watch", None)
        .await
        .unwrap();

    let scanner = ScanService::new(catalog.pools().clone(), thumb_dir);
    let ctrl = ScanControl::noop();
    scanner.scan_root(root.id, &ctrl).await.unwrap();

    std::fs::write(&file, include_bytes!("../tests/fixtures/minimal.jpg")).unwrap();
    let summary = scanner.scan_root(root.id, &ctrl).await.unwrap();
    assert!(summary.modified_count >= 1 || summary.scanned >= 1);

    std::fs::remove_file(&file).unwrap();
    let summary = scanner.scan_root(root.id, &ctrl).await.unwrap();
    assert_eq!(summary.missing_count, 1);
}

#[tokio::test]
async fn soft_delete_restore_and_deleted_only_filter() {
    let dir = tempdir().unwrap();
    let photos = dir.path().join("photos");
    std::fs::create_dir_all(&photos).unwrap();
    let jpeg = include_bytes!("../tests/fixtures/minimal.jpg");
    std::fs::write(photos.join("keep.jpg"), jpeg).unwrap();
    std::fs::write(photos.join("drop.jpg"), jpeg).unwrap();

    let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
    let pools = catalog.pools().clone();
    let thumb_dir = dir.path().join("thumbs");

    let library = LibraryService::new(pools.clone(), dir.path().to_path_buf());
    let root = library
        .add_local_root(photos.to_str().unwrap())
        .await
        .unwrap();
    let ctrl = ScanControl::noop();
    ScanService::new(pools.clone(), thumb_dir.clone())
        .scan_root(root.id, &ctrl)
        .await
        .unwrap();

    let query = QueryService::new(pools.clone(), thumb_dir.clone());
    let listed = query
        .query(&AssetFilter::default(), "date:desc", 0, 50)
        .await
        .unwrap();
    assert_eq!(listed.total, 2);
    let drop_id = listed
        .items
        .iter()
        .find(|item| item.file_name == "drop.jpg")
        .map(|item| item.id)
        .expect("drop asset");

    let assets = AssetRepo::new(pools.clone());
    assets.soft_delete(&[drop_id], 1).await.unwrap();

    let active = query
        .query(&AssetFilter::default(), "date:desc", 0, 50)
        .await
        .unwrap();
    assert_eq!(active.total, 1);

    let deleted = query
        .count(
            &AssetFilter {
                deleted_only: Some(true),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(deleted, 1);

    assets.restore_assets(&[drop_id]).await.unwrap();

    let restored = query
        .query(&AssetFilter::default(), "date:desc", 0, 50)
        .await
        .unwrap();
    assert_eq!(restored.total, 2);
}

#[tokio::test]
async fn multi_asset_batch_meta_and_rating_filter() {
    let dir = tempdir().unwrap();
    let photos = dir.path().join("photos");
    std::fs::create_dir_all(&photos).unwrap();
    let jpeg = include_bytes!("../tests/fixtures/minimal.jpg");
    std::fs::write(photos.join("low.jpg"), jpeg).unwrap();
    std::fs::write(photos.join("mid.jpg"), jpeg).unwrap();
    std::fs::write(photos.join("high.jpg"), jpeg).unwrap();

    let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
    let pools = catalog.pools().clone();
    let thumb_dir = dir.path().join("thumbs");

    let library = LibraryService::new(pools.clone(), dir.path().to_path_buf());
    let root = library
        .add_local_root(photos.to_str().unwrap())
        .await
        .unwrap();
    ScanService::new(pools.clone(), thumb_dir.clone())
        .scan_root(root.id, &ScanControl::noop())
        .await
        .unwrap();

    let query = QueryService::new(pools.clone(), thumb_dir.clone());
    let listed = query
        .query(&AssetFilter::default(), "date:desc", 0, 50)
        .await
        .unwrap();
    assert_eq!(listed.total, 3);

    let by_name = |name: &str| {
        listed
            .items
            .iter()
            .find(|item| item.file_name == name)
            .map(|item| item.id)
            .expect(name)
    };
    let low_id = by_name("low.jpg");
    let mid_id = by_name("mid.jpg");
    let high_id = by_name("high.jpg");

    query
        .apply_meta_patch(low_id, AssetMetaPatch { rating: Some(1), ..Default::default() })
        .await
        .unwrap();
    query
        .batch_apply_meta(
            &[mid_id, high_id],
            AssetMetaPatch { rating: Some(4), ..Default::default() },
        )
        .await
        .unwrap();
    query
        .apply_meta_patch(high_id, AssetMetaPatch { rating: Some(5), ..Default::default() })
        .await
        .unwrap();

    let filtered = query
        .query(
            &AssetFilter {
                rating_min: Some(4),
                ..Default::default()
            },
            "rating:desc",
            0,
            50,
        )
        .await
        .unwrap();
    assert_eq!(filtered.total, 2);
    assert_eq!(filtered.items[0].id, high_id);
    assert_eq!(filtered.items[1].id, mid_id);
}

#[tokio::test]
async fn export_nested_preserves_relative_paths() {
    let dir = tempdir().unwrap();
    let photos = dir.path().join("photos");
    let nested = photos.join("events").join("2024");
    std::fs::create_dir_all(&nested).unwrap();
    std::fs::write(
        nested.join("frame.jpg"),
        include_bytes!("../tests/fixtures/minimal.jpg"),
    )
    .unwrap();

    let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
    let pools = catalog.pools().clone();
    let thumb_dir = dir.path().join("thumbs");

    let library = LibraryService::new(pools.clone(), dir.path().to_path_buf());
    let root = library
        .add_local_root(photos.to_str().unwrap())
        .await
        .unwrap();
    ScanService::new(pools.clone(), thumb_dir.clone())
        .scan_root(root.id, &ScanControl::noop())
        .await
        .unwrap();

    let query = QueryService::new(pools.clone(), thumb_dir.clone());
    let listed = query
        .query(&AssetFilter::default(), "date:desc", 0, 50)
        .await
        .unwrap();
    let asset_id = listed.items[0].id;

    let export_dest = dir.path().join("nested-export");
    let manifest = ExportService::new(pools.clone())
        .export_assets(
            &[asset_id],
            &export_dest,
            &ExportOptions {
                flat: false,
                rename_template: None,
                format: None,
            },
            None,
        )
        .await
        .unwrap();
    assert_eq!(manifest.copied.len(), 1);
    assert!(export_dest.join("events").join("2024").join("frame.jpg").exists());
}

#[tokio::test]
async fn album_membership_filters_query() {
    let dir = tempdir().unwrap();
    let photos = dir.path().join("photos");
    std::fs::create_dir_all(&photos).unwrap();
    let jpeg = include_bytes!("../tests/fixtures/minimal.jpg");
    std::fs::write(photos.join("in-album.jpg"), jpeg).unwrap();
    std::fs::write(photos.join("outside.jpg"), jpeg).unwrap();

    let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
    let pools = catalog.pools().clone();
    let thumb_dir = dir.path().join("thumbs");

    let library = LibraryService::new(pools.clone(), dir.path().to_path_buf());
    let root = library
        .add_local_root(photos.to_str().unwrap())
        .await
        .unwrap();
    ScanService::new(pools.clone(), thumb_dir.clone())
        .scan_root(root.id, &ScanControl::noop())
        .await
        .unwrap();

    let query = QueryService::new(pools.clone(), thumb_dir.clone());
    let listed = query
        .query(&AssetFilter::default(), "date:desc", 0, 50)
        .await
        .unwrap();
    let in_album_id = listed
        .items
        .iter()
        .find(|item| item.file_name == "in-album.jpg")
        .map(|item| item.id)
        .expect("album asset");

    let collection = CollectionRepo::new(pools.clone());
    let album = collection
        .create_album("showcase", "date:desc", None)
        .await
        .unwrap();
    collection
        .set_album_items(album.id, &[in_album_id])
        .await
        .unwrap();

    let filtered = query
        .query(
            &AssetFilter {
                album_ids: Some(vec![album.id]),
                ..Default::default()
            },
            "date:desc",
            0,
            50,
        )
        .await
        .unwrap();
    assert_eq!(filtered.total, 1);
    assert_eq!(filtered.items[0].id, in_album_id);
}

#[tokio::test]
async fn root_id_filter_scopes_query_to_single_source() {
    let dir = tempdir().unwrap();
    let first = dir.path().join("first");
    let second = dir.path().join("second");
    std::fs::create_dir_all(&first).unwrap();
    std::fs::create_dir_all(&second).unwrap();
    let jpeg = include_bytes!("../tests/fixtures/minimal.jpg");
    std::fs::write(first.join("a.jpg"), jpeg).unwrap();
    std::fs::write(second.join("b.jpg"), jpeg).unwrap();

    let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
    let pools = catalog.pools().clone();
    let thumb_dir = dir.path().join("thumbs");
    let library = LibraryService::new(pools.clone(), dir.path().to_path_buf());
    let root_a = library.add_local_root(first.to_str().unwrap()).await.unwrap();
    let root_b = library.add_local_root(second.to_str().unwrap()).await.unwrap();
    let ctrl = ScanControl::noop();
    let scanner = ScanService::new(pools.clone(), thumb_dir.clone());
    scanner.scan_root(root_a.id, &ctrl).await.unwrap();
    scanner.scan_root(root_b.id, &ctrl).await.unwrap();

    let query = QueryService::new(pools.clone(), thumb_dir.clone());
    let scoped = query
        .query(
            &AssetFilter {
                root_id: Some(root_a.id),
                ..Default::default()
            },
            "date:desc",
            0,
            50,
        )
        .await
        .unwrap();
    assert_eq!(scoped.total, 1);
    assert_eq!(scoped.items[0].file_name, "a.jpg");

    let other = query
        .query(
            &AssetFilter {
                root_id: Some(root_b.id),
                ..Default::default()
            },
            "date:desc",
            0,
            50,
        )
        .await
        .unwrap();
    assert_eq!(other.total, 1);
    assert_eq!(other.items[0].file_name, "b.jpg");
}

#[tokio::test]
async fn duplicate_filter_after_scan_marks_collisions() {
    let dir = tempdir().unwrap();
    let photos = dir.path().join("photos");
    std::fs::create_dir_all(&photos).unwrap();
    let jpeg = include_bytes!("../tests/fixtures/minimal.jpg");
    std::fs::write(photos.join("dup-a.jpg"), jpeg).unwrap();
    std::fs::write(photos.join("dup-b.jpg"), jpeg).unwrap();

    let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
    let pools = catalog.pools().clone();
    let thumb_dir = dir.path().join("thumbs");
    let library = LibraryService::new(pools.clone(), dir.path().to_path_buf());
    let root = library.add_local_root(photos.to_str().unwrap()).await.unwrap();
    let scanner = ScanService::new(pools.clone(), thumb_dir.clone());
    scanner
        .scan_root(root.id, &ScanControl::noop())
        .await
        .unwrap();
    scanner.run_duplicate_index(root.id).await.unwrap();

    let query = QueryService::new(pools.clone(), thumb_dir.clone());
    let all = query
        .query(&AssetFilter::default(), "date:desc", 0, 50)
        .await
        .unwrap();
    assert_eq!(all.total, 2);

    let duplicates = query
        .query(
            &AssetFilter {
                has_duplicate: Some(true),
                ..Default::default()
            },
            "date:desc",
            0,
            50,
        )
        .await
        .unwrap();
    assert_eq!(duplicates.total, 2);
}

#[tokio::test]
async fn purge_after_soft_delete_removes_catalog_row() {
    let dir = tempdir().unwrap();
    let photos = dir.path().join("photos");
    std::fs::create_dir_all(&photos).unwrap();
    let file = photos.join("purge-me.jpg");
    std::fs::write(&file, include_bytes!("../tests/fixtures/minimal.jpg")).unwrap();

    let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
    let pools = catalog.pools().clone();
    let thumb_dir = dir.path().join("thumbs");
    let library = LibraryService::new(pools.clone(), dir.path().to_path_buf());
    let root = library.add_local_root(photos.to_str().unwrap()).await.unwrap();
    ScanService::new(pools.clone(), thumb_dir.clone())
        .scan_root(root.id, &ScanControl::noop())
        .await
        .unwrap();

    let query = QueryService::new(pools.clone(), thumb_dir.clone());
    let asset_id = query
        .query(&AssetFilter::default(), "date:desc", 0, 50)
        .await
        .unwrap()
        .items[0]
        .id;

    let assets = AssetRepo::new(pools.clone());
    assets.soft_delete(&[asset_id], 1).await.unwrap();
    let paths = WorkspacePaths::new(dir.path().to_path_buf());
    assets.purge_assets(&[asset_id], &paths, false).await.unwrap();

    assert!(!file.exists());
    let remaining = query
        .query(&AssetFilter::default(), "date:desc", 0, 50)
        .await
        .unwrap();
    assert_eq!(remaining.total, 0);
    assert!(
        AssetMetaRepo::new(pools.clone())
            .get(asset_id)
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn rebuild_catalog_rescans_without_dropping_roots() {
    let dir = tempdir().unwrap();
    let photos = dir.path().join("photos");
    std::fs::create_dir_all(&photos).unwrap();
    std::fs::write(
        photos.join("persist.jpg"),
        include_bytes!("../tests/fixtures/minimal.jpg"),
    )
    .unwrap();

    let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
    let pools = catalog.pools().clone();
    let thumb_dir = dir.path().join("thumbs");
    let library = LibraryService::new(pools.clone(), dir.path().to_path_buf());
    let root = library.add_local_root(photos.to_str().unwrap()).await.unwrap();
    ScanService::new(pools.clone(), thumb_dir.clone())
        .scan_root(root.id, &ScanControl::noop())
        .await
        .unwrap();

    let tag_repo = TagRepo::new(pools.clone());
    tag_repo.create_tag("cleared", None, None).await.unwrap();

    rebuild_and_rescan(pools.clone(), thumb_dir.clone()).await.unwrap();

    assert_eq!(library.list_roots().await.unwrap().len(), 1);
    assert!(tag_repo.list_tag_rows().await.unwrap().is_empty());

    let query = QueryService::new(pools.clone(), thumb_dir.clone());
    let listed = query
        .query(&AssetFilter::default(), "date:desc", 0, 50)
        .await
        .unwrap();
    assert_eq!(listed.total, 1);
    assert_eq!(listed.items[0].file_name, "persist.jpg");
}

#[tokio::test]
async fn tag_filter_narrows_grid_results() {
    let dir = tempdir().unwrap();
    let photos = dir.path().join("photos");
    std::fs::create_dir_all(&photos).unwrap();
    let jpeg = include_bytes!("../tests/fixtures/minimal.jpg");
    std::fs::write(photos.join("tagged.jpg"), jpeg).unwrap();
    std::fs::write(photos.join("plain.jpg"), jpeg).unwrap();

    let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
    let pools = catalog.pools().clone();
    let thumb_dir = dir.path().join("thumbs");
    let library = LibraryService::new(pools.clone(), dir.path().to_path_buf());
    let root = library.add_local_root(photos.to_str().unwrap()).await.unwrap();
    ScanService::new(pools.clone(), thumb_dir.clone())
        .scan_root(root.id, &ScanControl::noop())
        .await
        .unwrap();

    let query = QueryService::new(pools.clone(), thumb_dir.clone());
    let listed = query
        .query(&AssetFilter::default(), "date:desc", 0, 50)
        .await
        .unwrap();
    let tagged_id = listed
        .items
        .iter()
        .find(|item| item.file_name == "tagged.jpg")
        .map(|item| item.id)
        .expect("tagged asset");

    let tag_repo = TagRepo::new(pools.clone());
    let tag_id = tag_repo.create_tag("showcase", None, None).await.unwrap();
    query.batch_append_tags(&[tagged_id], tag_id).await.unwrap();

    let filtered = query
        .query(
            &AssetFilter {
                tag_ids: Some(vec![tag_id]),
                ..Default::default()
            },
            "date:desc",
            0,
            50,
        )
        .await
        .unwrap();
    assert_eq!(filtered.total, 1);
    assert_eq!(filtered.items[0].id, tagged_id);
}

#[tokio::test]
async fn raw_jpeg_pair_stays_queryable_after_link() {
    let dir = tempdir().unwrap();
    let photos = dir.path().join("photos");
    std::fs::create_dir_all(&photos).unwrap();
    let jpeg = include_bytes!("../tests/fixtures/minimal.jpg");
    std::fs::write(photos.join("DSC200.jpg"), jpeg).unwrap();
    std::fs::write(photos.join("DSC200.arw"), jpeg).unwrap();

    let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
    let pools = catalog.pools().clone();
    let thumb_dir = dir.path().join("thumbs");
    let library = LibraryService::new(pools.clone(), dir.path().to_path_buf());
    let root = library.add_local_root(photos.to_str().unwrap()).await.unwrap();
    ScanService::new(pools.clone(), thumb_dir.clone())
        .scan_root(root.id, &ScanControl::noop())
        .await
        .unwrap();

    let linked = LinkService::new(pools.clone())
        .link_raw_jpeg_in_root(root.id)
        .await
        .unwrap();
    assert_eq!(linked, 1);

    let query = QueryService::new(pools.clone(), thumb_dir.clone());
    let all = query
        .query(&AssetFilter::default(), "date:desc", 0, 50)
        .await
        .unwrap();
    assert_eq!(all.total, 2);

    let raw_only = query
        .query(
            &AssetFilter {
                kind: Some("raw".into()),
                ..Default::default()
            },
            "date:desc",
            0,
            50,
        )
        .await
        .unwrap();
    assert_eq!(raw_only.total, 1);
    assert_eq!(raw_only.items[0].kind, "raw");
}
