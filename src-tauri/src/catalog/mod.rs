pub mod models;
pub mod rebuild;
pub mod repo;
pub mod tables;

use crate::catalog::repo::{AssetMetaRepo, AssetRepo, RawTagRepo, SourceRootRepo, TagRepo};
use crate::error::Result;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use std::path::Path;
use std::str::FromStr;

#[derive(Clone)]
pub struct Catalog {
    pool: SqlitePool,
}

fn ensure_db_parent_directory(db_path: &Path) -> Result<()> {
    let parent = db_path.parent();
    if parent.is_none() {
        return Ok(());
    }
    std::fs::create_dir_all(parent.expect("parent"))?;
    Ok(())
}

impl Catalog {
    pub fn from_pool(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn open(db_path: &Path) -> Result<Self> {
        ensure_db_parent_directory(db_path)?;

        let options = SqliteConnectOptions::from_str(&format!("sqlite:{}", db_path.display()))?
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .foreign_keys(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await?;

        sqlx::migrate!("./migrations").run(&pool).await?;

        Ok(Self { pool })
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub fn assets(&self) -> AssetRepo {
        AssetRepo::new(self.pool.clone())
    }

    pub fn asset_meta(&self) -> AssetMetaRepo {
        AssetMetaRepo::new(self.pool.clone())
    }

    pub fn raw_tags(&self) -> RawTagRepo {
        RawTagRepo::new(self.pool.clone())
    }

    pub fn roots(&self) -> SourceRootRepo {
        SourceRootRepo::new(self.pool.clone())
    }

    pub fn tags(&self) -> TagRepo {
        TagRepo::new(self.pool.clone())
    }

    pub fn collection(&self) -> crate::collection::CollectionRepo {
        crate::collection::CollectionRepo::new(self.pool.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::repo::SourceRootRepo;
    use tempfile::tempdir;

    #[test]
    fn ensure_db_parent_directory_noops_without_parent() {
        ensure_db_parent_directory(Path::new("")).unwrap();
    }

    #[test]
    fn ensure_db_parent_directory_creates_nested_parents() {
        let dir = tempdir().unwrap();
        let nested = dir.path().join("nested/db");
        ensure_db_parent_directory(&nested.join("catalog.db")).unwrap();
        assert!(nested.is_dir());
    }

    #[tokio::test]
    async fn open_creates_schema() {
        let dir = tempdir().unwrap();
        let db = dir.path().join("test.db");
        let catalog = Catalog::open(&db).await.unwrap();

        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM source_root")
            .fetch_one(catalog.pool())
            .await
            .unwrap();
        assert_eq!(count.0, 0);
    }

    #[tokio::test]
    async fn accessors_return_repos() {
        let dir = tempdir().unwrap();
        let nested = dir.path().join("nested/dir");
        let catalog = Catalog::open(&nested.join("catalog.db")).await.unwrap();
        let root = catalog
            .roots()
            .insert_root("/photos", "local", "watch", None)
            .await
            .unwrap();
        assert_eq!(catalog.tags().list_tag_rows().await.unwrap().len(), 0);
        assert!(catalog.collection().list_albums().await.unwrap().is_empty());
        assert_eq!(catalog.roots().get_root(root.id).await.unwrap().id, root.id);
        assert!(catalog.asset_meta().get(root.id).await.unwrap().is_none());
        assert!(catalog
            .raw_tags()
            .list_for_asset(root.id)
            .await
            .unwrap()
            .is_empty());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn open_fails_when_db_parent_unwritable() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempdir().unwrap();
        let parent = dir.path().join("locked");
        std::fs::create_dir_all(&parent).unwrap();
        std::fs::set_permissions(&parent, std::fs::Permissions::from_mode(0o500)).unwrap();
        let db = parent.join("nested/catalog.db");
        let result = Catalog::open(&db).await;
        std::fs::set_permissions(&parent, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn insert_and_list_root() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("test.db")).await.unwrap();
        let repo = SourceRootRepo::new(catalog.pool().clone());

        let root = repo
            .insert_root("/photos", "local", "watch", None)
            .await
            .unwrap();
        assert_eq!(root.id, 1);
        assert_eq!(root.path, "/photos");

        let roots = repo.list_roots().await.unwrap();
        assert_eq!(roots.len(), 1);
    }
}
