use memhg_lib::catalog::models::AssetMetaPatch;
use memhg_lib::commands::asset::{
    batch_update_asset_meta, restore_assets, soft_delete_assets, update_asset_meta,
};
use memhg_lib::commands::collection::{create_album, set_album_items};
use memhg_lib::commands::query::{count_assets, query_assets};
use memhg_lib::query::AssetFilter;
use std::time::Duration;

use crate::common::{seed_extra_asset, seed_scanned_asset, wait_for_jobs_idle, TauriFixture};

#[tokio::test]
async fn multi_asset_batch_metadata_sort_and_album_filter() {
    let fixture = TauriFixture::new().await;
    let state = fixture.state();
    let handle = fixture.handle();
    let photos = fixture.hold.path().join("multi-meta");
    let (root_id, first_id) = seed_scanned_asset(&fixture, &photos, "alpha.jpg").await;
    let second_id = seed_extra_asset(&fixture, &photos, root_id, "beta.jpg").await;
    let third_id = seed_extra_asset(&fixture, &photos, root_id, "gamma.jpg").await;

    update_asset_meta(
        first_id,
        AssetMetaPatch { rating: Some(5) },
        handle.clone(),
        state.clone(),
    )
    .await
    .unwrap();
    let batch_count = batch_update_asset_meta(
        vec![second_id, third_id],
        AssetMetaPatch { rating: Some(3) },
        handle.clone(),
        state.clone(),
    )
    .await
    .unwrap();
    assert_eq!(batch_count, 2);
    wait_for_jobs_idle(state.clone(), Duration::from_secs(30)).await;

    let high_rated = query_assets(
        AssetFilter {
            rating_min: Some(5),
            ..Default::default()
        },
        Some("rating:desc".into()),
        Some(0),
        Some(10),
        state.clone(),
    )
    .await
    .unwrap();
    assert_eq!(high_rated.total, 1);
    assert_eq!(high_rated.items[0].id, first_id);

    let album = create_album(
        "pair".into(),
        Some("date:desc".into()),
        None,
        state.clone(),
    )
    .await
    .unwrap();
    set_album_items(album.id, vec![first_id, second_id], state.clone())
        .await
        .unwrap();

    let in_album = query_assets(
        AssetFilter {
            album_ids: Some(vec![album.id]),
            ..Default::default()
        },
        Some("date:desc".into()),
        Some(0),
        Some(10),
        state.clone(),
    )
    .await
    .unwrap();
    assert_eq!(in_album.total, 2);
    assert!(in_album.items.iter().any(|item| item.id == first_id));
    assert!(in_album.items.iter().any(|item| item.id == second_id));
    assert!(!in_album.items.iter().any(|item| item.id == third_id));
}

#[tokio::test]
async fn partial_soft_delete_restore_and_deleted_only_count() {
    let fixture = TauriFixture::new().await;
    let state = fixture.state();
    let handle = fixture.handle();
    let photos = fixture.hold.path().join("partial-restore");
    let (root_id, keep_id) = seed_scanned_asset(&fixture, &photos, "keep.jpg").await;
    let delete_id = seed_extra_asset(&fixture, &photos, root_id, "trash.jpg").await;

    let deleted = soft_delete_assets(vec![keep_id, delete_id], handle.clone(), state.clone())
        .await
        .unwrap();
    assert_eq!(deleted, 2);
    wait_for_jobs_idle(state.clone(), Duration::from_secs(30)).await;

    let active = count_assets(AssetFilter::default(), state.clone())
        .await
        .unwrap();
    assert_eq!(active, 0);

    let trashed = count_assets(
        AssetFilter {
            deleted_only: Some(true),
            ..Default::default()
        },
        state.clone(),
    )
    .await
    .unwrap();
    assert_eq!(trashed, 2);

    let restored = restore_assets(vec![keep_id], state.clone()).await.unwrap();
    assert_eq!(restored, 1);
    wait_for_jobs_idle(state.clone(), Duration::from_secs(30)).await;

    let active_after = count_assets(AssetFilter::default(), state.clone())
        .await
        .unwrap();
    assert_eq!(active_after, 1);

    let trashed_after = count_assets(
        AssetFilter {
            deleted_only: Some(true),
            ..Default::default()
        },
        state.clone(),
    )
    .await
    .unwrap();
    assert_eq!(trashed_after, 1);

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
