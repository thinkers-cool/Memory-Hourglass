use crate::catalog::models::SourceRoot;
use crate::catalog::pools::CatalogPools;
use crate::error::{AppError, Result};

pub struct SourceRootRepo {
    pools: CatalogPools,
}

struct InsertRootRequest<'a> {
    path: &'a str,
    kind: &'a str,
    scan_policy: &'a str,
    poll_secs: Option<i64>,
    smb_host: Option<&'a str>,
    smb_share: Option<&'a str>,
    smb_username: Option<&'a str>,
    smb_mounted: bool,
}

impl SourceRootRepo {
    pub fn new(pools: CatalogPools) -> Self {
        Self { pools }
    }

    pub async fn insert_root(
        &self,
        path: &str,
        kind: &str,
        scan_policy: &str,
        poll_secs: Option<i64>,
    ) -> Result<SourceRoot> {
        self.insert_root_with_smb(InsertRootRequest {
            path,
            kind,
            scan_policy,
            poll_secs,
            smb_host: None,
            smb_share: None,
            smb_username: None,
            smb_mounted: false,
        })
        .await
    }

    pub async fn insert_smb_mount_root(
        &self,
        path: &str,
        poll_secs: Option<i64>,
        host: &str,
        share: &str,
        username: &str,
    ) -> Result<SourceRoot> {
        self.insert_root_with_smb(InsertRootRequest {
            path,
            kind: "smb",
            scan_policy: "poll",
            poll_secs,
            smb_host: Some(host),
            smb_share: Some(share),
            smb_username: Some(username),
            smb_mounted: true,
        })
        .await
    }

    async fn insert_root_with_smb(&self, request: InsertRootRequest<'_>) -> Result<SourceRoot> {
        let InsertRootRequest {
            path,
            kind,
            scan_policy,
            poll_secs,
            smb_host,
            smb_share,
            smb_username,
            smb_mounted,
        } = request;
        let id = sqlx::query_scalar::<_, i64>(
            r#"
            INSERT INTO source_root (path, kind, scan_policy, poll_secs, status, smb_host, smb_share, smb_username, smb_mounted)
            VALUES (?, ?, ?, ?, 'idle', ?, ?, ?, ?)
            RETURNING id
            "#,
        )
        .bind(path)
        .bind(kind)
        .bind(scan_policy)
        .bind(poll_secs)
        .bind(smb_host)
        .bind(smb_share)
        .bind(smb_username)
        .bind(if smb_mounted { 1 } else { 0 })
        .fetch_one(self.pools.write())
        .await?;

        self.get_root(id).await
    }

    pub async fn get_root(&self, id: i64) -> Result<SourceRoot> {
        sqlx::query_as::<_, SourceRoot>("SELECT * FROM source_root WHERE id = ?")
            .bind(id)
            .fetch_optional(self.pools.read())
            .await?
            .ok_or_else(|| AppError::NotFound(format!("root {}", id)))
    }

    pub async fn list_roots(&self) -> Result<Vec<SourceRoot>> {
        Ok(
            sqlx::query_as::<_, SourceRoot>("SELECT * FROM source_root ORDER BY id")
                .fetch_all(self.pools.read())
                .await?,
        )
    }

    pub async fn remove_root(&self, id: i64) -> Result<()> {
        let result = sqlx::query("DELETE FROM source_root WHERE id = ?")
            .bind(id)
            .execute(self.pools.write())
            .await?;
        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("root {}", id)));
        }
        Ok(())
    }

    pub async fn set_status(&self, id: i64, status: &str) -> Result<()> {
        sqlx::query("UPDATE source_root SET status = ? WHERE id = ?")
            .bind(status)
            .bind(id)
            .execute(self.pools.write())
            .await?;
        Ok(())
    }

    pub async fn touch_scan(&self, id: i64, at: i64) -> Result<()> {
        sqlx::query("UPDATE source_root SET last_scan_at = ?, status = 'idle' WHERE id = ?")
            .bind(at)
            .bind(id)
            .execute(self.pools.write())
            .await?;
        Ok(())
    }

    pub async fn relink_path(&self, id: i64, path: &str) -> Result<SourceRoot> {
        let r = sqlx::query("UPDATE source_root SET path = ? WHERE id = ?")
            .bind(path)
            .bind(id)
            .execute(self.pools.write())
            .await?;
        if r.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("root {}", id)));
        }
        self.get_root(id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::repo::test_support::test_catalog;

    #[tokio::test]
    async fn source_root_not_found_errors() {
        let (catalog, _dir) = test_catalog().await;
        let roots = SourceRootRepo::new(catalog.pools().clone());
        let err = roots.get_root(999).await.unwrap_err();
        assert!(err.to_string().contains("not found"));
        let err = roots.remove_root(999).await.unwrap_err();
        assert!(err.to_string().contains("not found"));
        let err = roots.relink_path(999, "/tmp").await.unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[tokio::test]
    async fn insert_smb_mount_root_persists_smb_fields() {
        let (catalog, dir) = test_catalog().await;
        let roots = SourceRootRepo::new(catalog.pools().clone());
        let smb_path = dir.path().join("smb-share");
        std::fs::create_dir_all(&smb_path).unwrap();
        let root = roots
            .insert_smb_mount_root(
                smb_path.to_str().unwrap(),
                Some(120),
                "nas",
                "photos",
                "user",
            )
            .await
            .unwrap();
        assert_eq!(root.kind, "smb");
        assert_eq!(root.smb_host.as_deref(), Some("nas"));
        assert_eq!(root.smb_share.as_deref(), Some("photos"));
        assert_eq!(root.smb_username.as_deref(), Some("user"));
        assert_eq!(root.smb_mounted, 1);
    }

    #[tokio::test]
    async fn relink_path_updates_root_path() {
        let (catalog, _dir) = test_catalog().await;
        let roots = SourceRootRepo::new(catalog.pools().clone());
        let root = roots
            .insert_root("/tmp/old", "local", "watch", None)
            .await
            .unwrap();
        let updated = roots.relink_path(root.id, "/tmp/new").await.unwrap();
        assert_eq!(updated.path, "/tmp/new");
    }

    #[tokio::test]
    async fn remove_root_deletes_existing_root() {
        let (catalog, _dir) = test_catalog().await;
        let roots = SourceRootRepo::new(catalog.pools().clone());
        let root = roots
            .insert_root("/tmp/remove", "local", "watch", None)
            .await
            .unwrap();
        roots.remove_root(root.id).await.unwrap();
        assert!(roots.list_roots().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn set_status_and_touch_scan_update_root_fields() {
        let (catalog, _dir) = test_catalog().await;
        let roots = SourceRootRepo::new(catalog.pools().clone());
        let root = roots
            .insert_root("/tmp/status", "local", "watch", None)
            .await
            .unwrap();
        roots.set_status(root.id, "scanning").await.unwrap();
        roots.touch_scan(root.id, 12345).await.unwrap();
        let updated = roots.get_root(root.id).await.unwrap();
        assert_eq!(updated.status, "idle");
        assert_eq!(updated.last_scan_at, Some(12345));
    }

    #[tokio::test]
    async fn list_roots_returns_inserted_roots() {
        let (catalog, _dir) = test_catalog().await;
        let roots = SourceRootRepo::new(catalog.pools().clone());
        let root = roots
            .insert_root("/tmp/list", "local", "watch", None)
            .await
            .unwrap();
        let listed = roots.list_roots().await.unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, root.id);
    }

    #[tokio::test]
    async fn source_root_repo_errors_after_pool_close() {
        let (catalog, _dir) = test_catalog().await;
        let pools = catalog.pools().clone();
        let roots = SourceRootRepo::new(pools.clone());
        let root = roots
            .insert_root("/tmp/p", "local", "watch", None)
            .await
            .unwrap();
        pools.close().await;
        assert!(roots.get_root(root.id).await.is_err());
        assert!(roots.list_roots().await.is_err());
        assert!(roots.remove_root(root.id).await.is_err());
        assert!(roots.set_status(root.id, "idle").await.is_err());
        assert!(roots.touch_scan(root.id, 1).await.is_err());
        assert!(roots.relink_path(root.id, "/tmp/new").await.is_err());
        assert!(roots
            .insert_root("/tmp/other", "local", "watch", None)
            .await
            .is_err());
    }
}
