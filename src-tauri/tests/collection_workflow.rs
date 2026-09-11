use memhg_lib::collection::CollectionRepo;
use memhg_lib::catalog::Catalog;
use memhg_lib::query::AssetFilter;
use tempfile::tempdir;

#[tokio::test]
async fn smart_collection_roundtrip() {
    let dir = tempdir().unwrap();
    let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
    let collection = CollectionRepo::new(catalog.pool().clone());

    let filter = AssetFilter {
        rating_min: Some(4),
        kind: Some("image".into()),
        ..Default::default()
    };
    let saved = collection.save_smart_collection("favorites", &filter).await.unwrap();
    assert_eq!(saved.name, "favorites");

    let loaded = collection.get_smart_collection(saved.id).await.unwrap();
    assert_eq!(loaded.filter.rating_min, Some(4));
    assert_eq!(loaded.filter.kind.as_deref(), Some("image"));
}
