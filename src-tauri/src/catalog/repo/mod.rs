mod asset;
mod asset_meta;
mod asset_purge;
mod raw_tag;
mod source_root;
mod tag;

pub use asset::{AssetRepo, UpsertAssetInput};
pub use asset_meta::AssetMetaRepo;
pub use raw_tag::RawTagRepo;
pub use source_root::SourceRootRepo;
pub use tag::{TagRepo, TagRow};

#[cfg(test)]
pub(crate) mod test_support {
    use crate::catalog::Catalog;
    use tempfile::TempDir;

    pub async fn test_catalog() -> (Catalog, TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("test.db")).await.unwrap();
        (catalog, dir)
    }
}
