use memhg_lib::state::AppState;
use memhg_lib::workspace;
use tempfile::tempdir;

#[tokio::test]
async fn workspace_create_open_and_scope_libraries() {
    let dir = tempdir().unwrap();
    let app_data = dir.path().join("app");
    let state = AppState::new(app_data).unwrap();

    let wedding = dir.path().join("Wedding");
    std::fs::create_dir_all(&wedding).unwrap();
    let info = {
        let mut workspaces = state.workspaces.write().await;
        workspaces.create(&wedding, false).unwrap()
    };
    state.open_workspace(std::path::Path::new(&info.path)).await.unwrap();

    let photos = dir.path().join("photos");
    std::fs::create_dir_all(&photos).unwrap();
    state
        .with_active(|ws| async move {
            let root = ws.library.add_local_root(photos.to_str().unwrap()).await.unwrap();
            assert_eq!(ws.library.list_roots().await.unwrap().len(), 1);
            assert_eq!(root.path, photos.canonicalize().unwrap().to_string_lossy());
            Ok(())
        })
        .await
        .unwrap();

    state.close_workspace().await.unwrap();

    let family = dir.path().join("Family");
    std::fs::create_dir_all(&family).unwrap();
    let other = {
        let mut workspaces = state.workspaces.write().await;
        workspaces.create(&family, false).unwrap()
    };
    state.open_workspace(std::path::Path::new(&other.path)).await.unwrap();

    state
        .with_active(|ws| async move {
            assert!(ws.library.list_roots().await.unwrap().is_empty());
            Ok(())
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn workspace_manifest_lives_at_root() {
    let dir = tempdir().unwrap();
    let workspace_root = dir.path().join("Demo");
    std::fs::create_dir_all(&workspace_root).unwrap();
    let info = workspace::init_workspace_at(&workspace_root, false).unwrap();
    let root = std::path::Path::new(&info.path);
    assert!(root.join(workspace::WORKSPACE_MANIFEST).is_file());
    assert!(root.join(workspace::THUMBS_DIR).is_dir());
    assert!(!root.join(".memhg").exists());
}

#[tokio::test]
async fn recent_registry_updates_on_open() {
    let dir = tempdir().unwrap();
    let app_data = dir.path().join("app");
    let state = AppState::new(app_data.clone()).unwrap();
    let workspace_root = dir.path().join("Recent");
    std::fs::create_dir_all(&workspace_root).unwrap();
    let info = workspace::init_workspace_at(&workspace_root, false).unwrap();

    state.open_workspace(std::path::Path::new(&info.path)).await.unwrap();

    let workspaces = state.workspaces.read().await;
    let recent = workspaces.list_recent();
    assert_eq!(recent.len(), 1);
    assert_eq!(recent[0].path, info.path);
    assert_eq!(workspaces.last_opened().as_deref(), Some(info.path.as_str()));
}

#[tokio::test]
async fn read_only_workspace_keeps_media_bytes_on_metadata_write() {
    use memhg_lib::catalog::models::AssetMetaPatch;
    use memhg_lib::metadata::workspace_sidecar_path;
    use memhg_lib::query::AssetFilter;
    use memhg_lib::scan::ScanControl;

    let dir = tempdir().unwrap();
    let app_data = dir.path().join("app");
    let state = AppState::new(app_data).unwrap();

    let ws_root = dir.path().join("readonly-ws");
    std::fs::create_dir_all(&ws_root).unwrap();
    let info = {
        let mut workspaces = state.workspaces.write().await;
        workspaces.create(&ws_root, true).unwrap()
    };
    assert!(info.read_only);
    assert!(ws_root.join(workspace::XMP_DIR).is_dir());

    let photos = dir.path().join("photos");
    std::fs::create_dir_all(&photos).unwrap();
    std::fs::write(
        photos.join("sample.jpg"),
        include_bytes!("../tests/fixtures/minimal.jpg"),
    )
    .unwrap();

    state
        .open_workspace(std::path::Path::new(&info.path))
        .await
        .unwrap();

    let (root_id, photo_path, asset_id, xmp_dir) = state
        .with_active(|ws| async move {
            let root = ws
                .library
                .add_local_root(photos.to_str().unwrap())
                .await
                .unwrap();
            let summary = ws
                .scan_service()
                .scan_root(root.id, &ScanControl::noop())
                .await
                .unwrap();
            assert_eq!(summary.indexed, 1);

            let listed = ws
                .query_service()
                .query(&AssetFilter::default(), "date:desc", 0, 50)
                .await
                .unwrap();
            let asset = &listed.items[0];
            Ok((
                root.id,
                asset.abs_path.clone(),
                asset.id,
                ws.paths.xmp_dir().clone(),
            ))
        })
        .await
        .unwrap();

    let before = std::fs::read(&photo_path).unwrap();

    state
        .with_active(|ws| async move {
            ws.query_service()
                .apply_meta_patch(
                    asset_id,
                    AssetMetaPatch {
                        rating: Some(4),
                    },
                )
                .await
                .unwrap();
            Ok(())
        })
        .await
        .unwrap();

    let after = std::fs::read(&photo_path).unwrap();
    assert_eq!(before, after);

    let sidecar = workspace_sidecar_path(&xmp_dir, root_id, "sample.jpg");
    assert!(sidecar.is_file());
}
