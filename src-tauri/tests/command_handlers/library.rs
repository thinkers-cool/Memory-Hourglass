use memhg_lib::catalog::Catalog;
use memhg_lib::commands::library::{
    add_root, add_smb_source, list_roots, preview_relink, relink_root, remove_root,
};
use memhg_lib::commands::scan::{get_scan_status, start_scan};
use memhg_lib::commands::smb::{list_smb_shares, mount_smb_for_browse};
use memhg_lib::library::{LibraryService, SmbSourceInput};
use memhg_lib::smb::{SmbConnectRequest, SmbListRequest};
use tempfile::tempdir;

use crate::common::{seed_scanned_asset, wait_for_scan, TauriFixture};

#[tokio::test]
async fn library_relink_and_smb_commands() {
    let fixture = TauriFixture::new().await;
    let state = fixture.state();
    let photos = fixture.hold.path().join("relink-src");
    let moved = fixture.hold.path().join("relink-dst");
    let (_root_id, _asset_id) = seed_scanned_asset(&fixture, &photos, "move-me.jpg").await;
    let roots = list_roots(state.clone()).await.unwrap();
    let root_id = roots[0].id;

    std::fs::create_dir_all(&moved).unwrap();
    std::fs::copy(photos.join("move-me.jpg"), moved.join("move-me.jpg")).unwrap();

    let preview = preview_relink(root_id, moved.to_string_lossy().to_string(), state.clone())
        .await
        .unwrap();
    assert_eq!(preview.matched, preview.total_sampled);

    relink_root(root_id, moved.to_string_lossy().to_string(), state.clone())
        .await
        .unwrap();

    let smb_dir = fixture.hold.path().join("smb-mounted");
    std::fs::create_dir_all(&smb_dir).unwrap();
    std::fs::write(
        smb_dir.join("net.jpg"),
        include_bytes!("../fixtures/minimal.jpg"),
    )
    .unwrap();
    let smb_root = add_smb_source(
        SmbSourceInput::Mounted {
            path: smb_dir.to_string_lossy().to_string(),
            poll_secs: Some(60),
        },
        state.clone(),
    )
    .await
    .unwrap();
    assert_eq!(smb_root.kind, "smb");

    let browse_mount = mount_smb_for_browse(
        SmbConnectRequest {
            host: "127.0.0.1".into(),
            share: "public".into(),
            username: "guest".into(),
            password: "".into(),
            domain: None,
            poll_secs: None,
            sub_path: None,
            read_only: false,
        },
        state.clone(),
    )
    .await;
    assert!(browse_mount.is_err());

    let shares = list_smb_shares(
        SmbListRequest {
            host: "127.0.0.1".into(),
            username: "guest".into(),
            password: "".into(),
        },
        state.clone(),
    )
    .await;
    assert!(shares.is_err());

    remove_root(root_id, state.clone()).await.unwrap();
    remove_root(smb_root.id, state.clone()).await.unwrap();
}

#[tokio::test]
async fn library_remove_root_cleans_smb_mount_metadata() {
    let dir = tempdir().unwrap();
    let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
    let pool = catalog.pool().clone();
    let smb_dir = dir.path().join("smb-local");
    std::fs::create_dir_all(&smb_dir).unwrap();
    let library = LibraryService::new(pool.clone(), dir.path().join("mounts"));
    let root = library
        .add_smb_root(smb_dir.to_str().unwrap(), Some(60))
        .await
        .expect("smb root");
    library.remove_root(root.id).await.expect("remove");
    assert!(library.list_roots().await.expect("roots").is_empty());
}

#[tokio::test]
async fn rescan_empty_root_completes_without_index_queue() {
    let fixture = TauriFixture::new().await;
    let state = fixture.state();
    let handle = fixture.handle();
    let empty = fixture.hold.path().join("empty-root");
    std::fs::create_dir_all(&empty).unwrap();
    let root = add_root(empty.to_string_lossy().to_string(), state.clone())
        .await
        .expect("add root");
    start_scan(root.id, handle, state.clone())
        .await
        .expect("scan");
    let status = wait_for_scan(state.clone(), std::time::Duration::from_secs(10)).await;
    assert_eq!(status.stage, "done");
    let scan_status = get_scan_status(state.clone()).await.expect("status");
    assert_eq!(scan_status.stage, "done");
}
