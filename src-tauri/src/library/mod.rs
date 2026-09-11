use crate::catalog::models::SourceRoot;
use crate::catalog::repo::{AssetRepo, SourceRootRepo};
use crate::error::{AppError, Result};
use crate::smb::{
    browse_mount_point, delete_credentials, ensure_share_mounted, is_mounted, load_credentials,
    mount_share, resolve_share_sub_path, share_mount_point, unmount_share, SmbConnectRequest,
};
use serde::Deserialize;
use serde::Serialize;
use sqlx::SqlitePool;
use std::path::{Path, PathBuf};

pub mod dirs;

pub use dirs::{
    list_child_directories, resolve_path_under_root, resolve_share_subfolder, validate_file_name,
    validate_rel_path, FolderEntry,
};

pub struct LibraryService {
    pool: SqlitePool,
    mount_dir: PathBuf,
    read_only: bool,
}

fn cleanup_smb_mount_for_root(
    mount_dir: &Path,
    host: &str,
    share: &str,
    username: &str,
) -> Result<()> {
    let share_mount = share_mount_point(mount_dir, host, share, username);
    if share_mount.exists() {
        unmount_share(&share_mount)?;
    }
    delete_credentials(host, share, username)
}

async fn validate_browsable_path(path: &str, mount_dir: &Path, pool: &SqlitePool) -> Result<()> {
    let canonical = std::fs::canonicalize(path)
        .map_err(|_| AppError::Library(format!("path not found: {}", path)))?;
    if mount_dir
        .canonicalize()
        .map(|mount_path| canonical.starts_with(&mount_path))
        .unwrap_or(false)
    {
        return Ok(());
    }
    let roots = SourceRootRepo::new(pool.clone()).list_roots().await?;
    for root in roots {
        if let Ok(root_path) = std::fs::canonicalize(&root.path) {
            if canonical.starts_with(&root_path) {
                return Ok(());
            }
        }
    }
    Err(AppError::InvalidInput(
        "path is outside allowed library locations".into(),
    ))
}

async fn relink_smb_root_if_needed(
    repo: &SourceRootRepo,
    root_id: i64,
    share_mount: &Path,
    share_sub_path: &str,
    current_path: &str,
) -> Result<()> {
    let Ok(library_path) = resolve_share_subfolder(
        share_mount,
        (!share_sub_path.is_empty()).then_some(share_sub_path),
    ) else {
        return Ok(());
    };
    let library_path = library_path.to_string_lossy().to_string();
    if library_path != current_path {
        repo.relink_path(root_id, &library_path).await?;
    }
    Ok(())
}

impl LibraryService {
    pub fn new(pool: SqlitePool, mount_dir: PathBuf) -> Self {
        Self::with_mount_mode(pool, mount_dir, false)
    }

    pub fn with_mount_mode(pool: SqlitePool, mount_dir: PathBuf, read_only: bool) -> Self {
        Self {
            pool,
            mount_dir,
            read_only,
        }
    }

    fn apply_mount_mode(&self, req: SmbConnectRequest) -> SmbConnectRequest {
        let mut req = req;
        req.read_only = self.read_only;
        req
    }

    pub async fn add_local_root(&self, path: &str) -> Result<SourceRoot> {
        let canonical = std::fs::canonicalize(path)
            .map_err(|_| AppError::Library(format!("path not found: {}", path)))?;
        if !canonical.is_dir() {
            return Err(AppError::Library("path is not a directory".into()));
        }

        let repo = SourceRootRepo::new(self.pool.clone());
        repo.insert_root(canonical.to_string_lossy().as_ref(), "local", "watch", None)
            .await
    }

    pub async fn add_smb_source(&self, input: SmbSourceInput) -> Result<SourceRoot> {
        match input {
            SmbSourceInput::Mounted { path, poll_secs } => {
                self.add_smb_root(&path, poll_secs).await
            }
            SmbSourceInput::Connect(req) => self.connect_smb_share(&req).await,
        }
    }

    pub async fn add_smb_root(&self, path: &str, poll_secs: Option<i64>) -> Result<SourceRoot> {
        let canonical = std::fs::canonicalize(path)
            .map_err(|_| AppError::Library(format!("path not found: {}", path)))?;
        if !canonical.is_dir() {
            return Err(AppError::Library("path is not a directory".into()));
        }

        let poll = Some(poll_secs.unwrap_or(300).max(30));
        let repo = SourceRootRepo::new(self.pool.clone());
        repo.insert_root(canonical.to_string_lossy().as_ref(), "smb", "poll", poll)
            .await
    }

    pub async fn mount_smb_for_browse(&self, req: &SmbConnectRequest) -> Result<String> {
        let req = self.apply_mount_mode(req.clone());
        let mount_path = browse_mount_point(&self.mount_dir, &req.host, &req.share, &req.username);
        let mount_req = req.clone();
        let mount_path_for_return = mount_path.clone();
        tokio::task::spawn_blocking(move || mount_share(&mount_path, &mount_req))
            .await
            .map_err(|error| AppError::Library(format!("SMB mount failed: {}", error)))??;
        Ok(mount_path_for_return.to_string_lossy().to_string())
    }

    pub async fn list_folder_children(&self, path: &str) -> Result<Vec<FolderEntry>> {
        validate_browsable_path(path, &self.mount_dir, &self.pool).await?;
        let path = path.to_string();
        tokio::task::spawn_blocking(move || list_child_directories(Path::new(&path)))
            .await
            .map_err(|error| AppError::Library(format!("folder listing failed: {}", error)))?
    }

    pub async fn connect_smb_share(&self, req: &SmbConnectRequest) -> Result<SourceRoot> {
        let req = self.apply_mount_mode(req.clone());
        let share_mount = share_mount_point(&self.mount_dir, &req.host, &req.share, &req.username);
        let mount_dir = self.mount_dir.clone();
        let mount_req = req.clone();
        tokio::task::spawn_blocking(move || ensure_share_mounted(&mount_dir, &mount_req))
            .await
            .map_err(|error| AppError::Library(format!("SMB mount failed: {}", error)))??;

        let library_path = resolve_share_subfolder(&share_mount, req.sub_path.as_deref())?;
        let poll = Some(req.poll_secs.unwrap_or(300).max(30));
        SourceRootRepo::new(self.pool.clone())
            .insert_smb_mount_root(
                library_path.to_string_lossy().as_ref(),
                poll,
                &req.host,
                &req.share,
                &req.username,
            )
            .await
    }

    pub async fn relink_root(&self, id: i64, path: &str) -> Result<SourceRoot> {
        let canonical = std::fs::canonicalize(path)
            .map_err(|_| AppError::Library(format!("path not found: {}", path)))?;
        if !canonical.is_dir() {
            return Err(AppError::Library("path is not a directory".into()));
        }

        SourceRootRepo::new(self.pool.clone())
            .relink_path(id, canonical.to_string_lossy().as_ref())
            .await
    }

    pub async fn remove_root(&self, id: i64) -> Result<()> {
        let repo = SourceRootRepo::new(self.pool.clone());
        let root = repo.get_root(id).await?;
        if root.smb_mounted != 0 {
            if let (Some(host), Some(share), Some(username)) =
                (&root.smb_host, &root.smb_share, &root.smb_username)
            {
                let roots = repo.list_roots().await?;
                let shared = roots.iter().any(|entry| {
                    entry.id != id
                        && entry.smb_mounted != 0
                        && entry.smb_host.as_deref() == Some(host.as_str())
                        && entry.smb_share.as_deref() == Some(share.as_str())
                        && entry.smb_username.as_deref() == Some(username.as_str())
                });
                if !shared {
                    cleanup_smb_mount_for_root(&self.mount_dir, host, share, username)?;
                }
            }
        }
        repo.remove_root(id).await
    }

    pub async fn list_roots(&self) -> Result<Vec<SourceRoot>> {
        SourceRootRepo::new(self.pool.clone()).list_roots().await
    }

    pub async fn ensure_smb_mounts_ready(&self) -> Result<()> {
        let repo = SourceRootRepo::new(self.pool.clone());
        let roots = repo.list_roots().await?;
        for root in &roots {
            if root.smb_mounted == 0 {
                continue;
            }
            let (host, share, username) = match (
                root.smb_host.as_deref(),
                root.smb_share.as_deref(),
                root.smb_username.as_deref(),
            ) {
                (Some(host), Some(share), Some(username)) => (host, share, username),
                _ => continue,
            };

            let root_path = Path::new(&root.path);
            let share_mount = share_mount_point(&self.mount_dir, host, share, username);
            let share_sub_path =
                resolve_share_sub_path(&self.mount_dir, root_path, host, share, username);

            let mount_ready = if is_mounted(&share_mount) {
                true
            } else {
                match load_credentials(host, share, username) {
                    Ok(password) => {
                        let req = self.apply_mount_mode(SmbConnectRequest {
                            host: host.to_string(),
                            share: share.to_string(),
                            username: username.to_string(),
                            password,
                            domain: None,
                            poll_secs: root.poll_secs,
                            sub_path: None,
                            read_only: false,
                        });
                        let mount_dir = self.mount_dir.clone();
                        let mount_result = tokio::task::spawn_blocking(move || {
                            ensure_share_mounted(&mount_dir, &req)
                        })
                        .await
                        .map_err(|error| AppError::Library(format!("SMB mount failed: {}", error)));
                        match mount_result {
                            Ok(Ok(())) => true,
                            Ok(Err(error)) => {
                                tracing::warn!(
                                    "SMB mount failed for root {} ({}): {}",
                                    root.id,
                                    root.path,
                                    error
                                );
                                false
                            }
                            Err(error) => {
                                tracing::warn!(
                                    "SMB mount task failed for root {} ({}): {}",
                                    root.id,
                                    root.path,
                                    error
                                );
                                false
                            }
                        }
                    }
                    Err(error) => {
                        tracing::warn!(
                            "SMB credentials missing for root {} ({}): {}",
                            root.id,
                            root.path,
                            error
                        );
                        false
                    }
                }
            };

            if mount_ready {
                relink_smb_root_if_needed(
                    &repo,
                    root.id,
                    &share_mount,
                    &share_sub_path,
                    &root.path,
                )
                .await?;
                repo.set_status(root.id, "idle").await?;
            } else {
                repo.set_status(root.id, "offline").await?;
            }
        }
        Ok(())
    }

    pub async fn list_root_stats(&self) -> Result<Vec<RootStats>> {
        let repo = SourceRootRepo::new(self.pool.clone());
        let assets = AssetRepo::new(self.pool.clone());
        let roots = repo.list_roots().await?;
        let mut stats = Vec::new();
        for root in roots {
            let (asset_count, missing_count) = assets.count_for_root(root.id).await?;
            stats.push(RootStats {
                id: root.id,
                path: root.path,
                kind: root.kind,
                status: root.status,
                scan_policy: root.scan_policy,
                poll_secs: root.poll_secs,
                last_scan_at: root.last_scan_at,
                asset_count,
                missing_count,
            });
        }
        Ok(stats)
    }

    pub async fn preview_relink(&self, id: i64, new_path: &str) -> Result<RelinkPreview> {
        let repo = SourceRootRepo::new(self.pool.clone());
        let assets = AssetRepo::new(self.pool.clone());
        let root = repo.get_root(id).await?;
        let old_base = std::path::PathBuf::from(&root.path);
        let new_base = std::fs::canonicalize(new_path)
            .map_err(|_| AppError::Library(format!("path not found: {}", new_path)))?;

        let paths = assets.list_paths_for_root(id).await?;
        let sample = paths.iter().take(20);
        let mut matched = 0u64;
        let mut total = 0u64;
        for (rel, _, _) in sample {
            total += 1;
            if new_base.join(rel).exists() || old_base.join(rel).exists() {
                matched += 1;
            }
        }
        Ok(RelinkPreview {
            matched,
            total_sampled: total,
            new_path: new_base.to_string_lossy().to_string(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum SmbSourceInput {
    Mounted {
        path: String,
        poll_secs: Option<i64>,
    },
    Connect(SmbConnectRequest),
}

#[derive(Debug, Clone, Serialize)]
pub struct RootStats {
    pub id: i64,
    pub path: String,
    pub kind: String,
    pub status: String,
    pub scan_policy: String,
    pub poll_secs: Option<i64>,
    pub last_scan_at: Option<i64>,
    pub asset_count: i64,
    pub missing_count: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct RelinkPreview {
    pub matched: u64,
    pub total_sampled: u64,
    pub new_path: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::Catalog;
    use crate::scan::{ScanControl, ScanService};
    use tempfile::tempdir;

    #[tokio::test]
    async fn preview_relink_counts_matches() {
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        std::fs::write(
            photos.join("a.jpg"),
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();

        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let library = LibraryService::new(catalog.pool().clone(), dir.path().to_path_buf());
        let root = library
            .add_local_root(photos.to_str().unwrap())
            .await
            .unwrap();
        ScanService::new(catalog.pool().clone(), dir.path().join("thumbs"))
            .scan_root(root.id, &ScanControl::noop())
            .await
            .unwrap();

        let moved = dir.path().join("moved");
        std::fs::create_dir_all(&moved).unwrap();
        std::fs::copy(photos.join("a.jpg"), moved.join("a.jpg")).unwrap();

        let preview = library
            .preview_relink(root.id, moved.to_str().unwrap())
            .await
            .unwrap();
        assert_eq!(preview.matched, 1);
        assert_eq!(preview.total_sampled, 1);
    }

    #[tokio::test]
    async fn add_and_remove_root() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let library = LibraryService::new(catalog.pool().clone(), dir.path().to_path_buf());

        let root = library
            .add_local_root(dir.path().to_str().unwrap())
            .await
            .unwrap();
        assert!(root.path.contains(dir.path().to_str().unwrap()));

        library.remove_root(root.id).await.unwrap();
        assert!(library.list_roots().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn ensure_smb_mounts_ready_allows_missing_credentials() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let repo = SourceRootRepo::new(catalog.pool().clone());
        let root = repo
            .insert_smb_mount_root(
                "/missing/smb/path",
                Some(300),
                "192.168.1.1",
                "Download",
                "tonysu",
            )
            .await
            .unwrap();
        let library = LibraryService::new(catalog.pool().clone(), dir.path().join("mounts"));
        library.ensure_smb_mounts_ready().await.unwrap();
        let updated = repo.get_root(root.id).await.unwrap();
        assert_eq!(updated.status, "offline");
    }

    #[tokio::test]
    async fn add_local_root_rejects_missing_path() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let library = LibraryService::new(catalog.pool().clone(), dir.path().to_path_buf());
        assert!(library.add_local_root("/missing/path").await.is_err());
    }

    #[tokio::test]
    async fn add_local_root_rejects_file_path() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("file.txt");
        std::fs::write(&file, b"x").unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let library = LibraryService::new(catalog.pool().clone(), dir.path().to_path_buf());
        assert!(library
            .add_local_root(file.to_str().unwrap())
            .await
            .is_err());
    }

    #[tokio::test]
    async fn list_folder_children_returns_directories() {
        let dir = tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("nested")).unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let library = LibraryService::new(catalog.pool().clone(), dir.path().to_path_buf());
        let entries = library
            .list_folder_children(dir.path().to_str().unwrap())
            .await
            .unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "nested");
    }

    #[tokio::test]
    async fn connect_smb_share_registers_root_when_share_is_mounted() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let mount_dir = dir.path().join("mounts");
        let host = format!("connect-{}", std::process::id());
        let share_mount = share_mount_point(&mount_dir, &host, "photos", "guest");
        std::fs::create_dir_all(share_mount.join("nested")).unwrap();
        std::fs::write(share_mount.join(crate::smb::TEST_MOUNT_MARKER), b"1").unwrap();
        let library = LibraryService::new(catalog.pool().clone(), mount_dir);
        let root = library
            .connect_smb_share(&SmbConnectRequest {
                host,
                share: "photos".into(),
                username: "guest".into(),
                password: "secret".into(),
                domain: None,
                poll_secs: Some(60),
                sub_path: Some("nested".into()),
                read_only: false,
            })
            .await
            .unwrap();
        assert_eq!(root.kind, "smb");
        assert!(root.path.contains("nested"));
    }

    #[tokio::test]
    async fn ensure_smb_mounts_ready_remounts_with_stored_credentials() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let repo = SourceRootRepo::new(catalog.pool().clone());
        let mount_dir = dir.path().join("mounts");
        let host = format!("remount-{}", std::process::id());
        let share_mount = share_mount_point(&mount_dir, &host, "photos", "user");
        std::fs::create_dir_all(share_mount.join("library")).unwrap();
        std::fs::write(share_mount.join(crate::smb::TEST_MOUNT_MARKER), b"1").unwrap();
        crate::smb::store_credentials(&host, "photos", "user", "secret").unwrap();
        let root = repo
            .insert_smb_mount_root(
                share_mount.join("library").to_string_lossy().as_ref(),
                Some(300),
                &host,
                "photos",
                "user",
            )
            .await
            .unwrap();
        let library = LibraryService::new(catalog.pool().clone(), mount_dir);
        library.ensure_smb_mounts_ready().await.unwrap();
        let updated = repo.get_root(root.id).await.unwrap();
        assert_eq!(updated.status, "idle");
    }

    #[tokio::test]
    async fn remove_root_unmounts_smb_when_last_share_user() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let mount_dir = dir.path().join("mounts");
        let host = format!("remove-{}", std::process::id());
        let share_mount = share_mount_point(&mount_dir, &host, "photos", "user");
        std::fs::create_dir_all(&share_mount).unwrap();
        std::fs::write(share_mount.join(crate::smb::TEST_MOUNT_MARKER), b"1").unwrap();
        crate::smb::store_credentials(&host, "photos", "user", "secret").unwrap();
        let library = LibraryService::new(catalog.pool().clone(), mount_dir);
        let root = library
            .add_smb_root(share_mount.to_str().unwrap(), Some(60))
            .await
            .unwrap();
        sqlx::query("UPDATE source_root SET smb_mounted = 1, smb_host = ?, smb_share = ?, smb_username = ? WHERE id = ?")
            .bind(&host)
            .bind("photos")
            .bind("user")
            .bind(root.id)
            .execute(catalog.pool())
            .await
            .unwrap();
        library.remove_root(root.id).await.unwrap();
        assert!(library.list_roots().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn list_root_stats_reports_asset_counts() {
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        std::fs::write(
            photos.join("a.jpg"),
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let library = LibraryService::new(catalog.pool().clone(), dir.path().to_path_buf());
        let root = library
            .add_local_root(photos.to_str().unwrap())
            .await
            .unwrap();
        ScanService::new(catalog.pool().clone(), dir.path().join("thumbs"))
            .scan_root(root.id, &ScanControl::noop())
            .await
            .unwrap();
        let stats = library.list_root_stats().await.unwrap();
        assert_eq!(stats[0].asset_count, 1);
    }

    #[tokio::test]
    async fn add_smb_source_connect_registers_root() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let mount_dir = dir.path().join("mounts");
        let host = format!("source-connect-{}", std::process::id());
        let share_mount = share_mount_point(&mount_dir, &host, "photos", "guest");
        std::fs::create_dir_all(share_mount.join("album")).unwrap();
        std::fs::write(share_mount.join(crate::smb::TEST_MOUNT_MARKER), b"1").unwrap();
        let library = LibraryService::new(catalog.pool().clone(), mount_dir);
        let root = library
            .add_smb_source(SmbSourceInput::Connect(SmbConnectRequest {
                host,
                share: "photos".into(),
                username: "guest".into(),
                password: "secret".into(),
                domain: None,
                poll_secs: Some(60),
                sub_path: Some("album".into()),
                read_only: false,
            }))
            .await
            .unwrap();
        assert_eq!(root.kind, "smb");
        assert!(root.path.contains("album"));
    }

    #[tokio::test]
    async fn remove_root_keeps_shared_smb_mount() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let mount_dir = dir.path().join("mounts");
        let host = format!("shared-{}", std::process::id());
        let share_mount = share_mount_point(&mount_dir, &host, "photos", "user");
        std::fs::create_dir_all(share_mount.join("lib-a")).unwrap();
        std::fs::create_dir_all(share_mount.join("lib-b")).unwrap();
        std::fs::write(share_mount.join(crate::smb::TEST_MOUNT_MARKER), b"1").unwrap();
        crate::smb::store_credentials(&host, "photos", "user", "secret").unwrap();
        let repo = SourceRootRepo::new(catalog.pool().clone());
        let root_a = repo
            .insert_smb_mount_root(
                share_mount.join("lib-a").to_string_lossy().as_ref(),
                Some(300),
                &host,
                "photos",
                "user",
            )
            .await
            .unwrap();
        let root_b = repo
            .insert_smb_mount_root(
                share_mount.join("lib-b").to_string_lossy().as_ref(),
                Some(300),
                &host,
                "photos",
                "user",
            )
            .await
            .unwrap();
        let library = LibraryService::new(catalog.pool().clone(), mount_dir.clone());
        library.remove_root(root_a.id).await.unwrap();
        assert!(share_mount.join(crate::smb::TEST_MOUNT_MARKER).is_file());
        library.remove_root(root_b.id).await.unwrap();
        assert!(library.list_roots().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn remove_root_skips_unmount_when_smb_metadata_missing() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let library = LibraryService::new(catalog.pool().clone(), dir.path().join("mounts"));
        let root = library
            .add_local_root(dir.path().to_str().unwrap())
            .await
            .unwrap();
        sqlx::query("UPDATE source_root SET smb_mounted = 1 WHERE id = ?")
            .bind(root.id)
            .execute(catalog.pool())
            .await
            .unwrap();
        library.remove_root(root.id).await.unwrap();
        assert!(library.list_roots().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn ensure_smb_mounts_ready_marks_offline_when_mount_fails() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let repo = SourceRootRepo::new(catalog.pool().clone());
        let host = format!("mount-fail-{}", std::process::id());
        crate::smb::store_credentials(&host, "photos", "user", "secret").unwrap();
        let root = repo
            .insert_smb_mount_root("/missing/smb/library", Some(300), &host, "photos", "user")
            .await
            .unwrap();
        let library = LibraryService::new(catalog.pool().clone(), dir.path().join("mounts"));
        library.ensure_smb_mounts_ready().await.unwrap();
        let updated = repo.get_root(root.id).await.unwrap();
        assert_eq!(updated.status, "offline");
    }

    #[tokio::test]
    async fn ensure_smb_mounts_ready_relinks_when_stored_path_differs() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let repo = SourceRootRepo::new(catalog.pool().clone());
        let mount_dir = dir.path().join("mounts");
        let host = format!("relink-{}", std::process::id());
        let share_mount = share_mount_point(&mount_dir, &host, "photos", "user");
        let library_dir = share_mount.join("library");
        std::fs::create_dir_all(&library_dir).unwrap();
        std::fs::write(share_mount.join(crate::smb::TEST_MOUNT_MARKER), b"1").unwrap();
        crate::smb::store_credentials(&host, "photos", "user", "secret").unwrap();
        let stored_path = format!("{}/library/", share_mount.to_string_lossy());
        let root = repo
            .insert_smb_mount_root(&stored_path, Some(300), &host, "photos", "user")
            .await
            .unwrap();
        let library = LibraryService::new(catalog.pool().clone(), mount_dir);
        library.ensure_smb_mounts_ready().await.unwrap();
        let updated = repo.get_root(root.id).await.unwrap();
        assert_eq!(updated.status, "idle");
        assert_ne!(updated.path, stored_path);
        assert!(updated.path.ends_with("library"));
    }

    #[tokio::test]
    async fn add_smb_root_and_relink_validate_paths() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let library = LibraryService::new(catalog.pool().clone(), dir.path().join("mounts"));
        let file = dir.path().join("file.txt");
        std::fs::write(&file, b"x").unwrap();
        assert!(library
            .add_smb_root(file.to_str().unwrap(), None)
            .await
            .is_err());
        assert!(library.relink_root(999, "/missing").await.is_err());

        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        let root = library
            .add_smb_root(photos.to_str().unwrap(), Some(15))
            .await
            .unwrap();
        assert_eq!(root.poll_secs, Some(30));
        let moved = dir.path().join("moved");
        std::fs::create_dir_all(&moved).unwrap();
        let relinked = library
            .relink_root(root.id, moved.to_str().unwrap())
            .await
            .unwrap();
        assert!(relinked.path.contains("moved"));
    }

    #[cfg(target_os = "macos")]
    #[tokio::test]
    async fn read_only_workspace_mounts_smb_with_ro_option() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let mount_dir = dir.path().join("mounts");
        std::fs::create_dir_all(&mount_dir).unwrap();
        let args_log = mount_dir.join("mount-args.txt");
        let security = mount_dir.join("security.sh");
        crate::test_support::unix::write_executable(&security, "#!/bin/sh\nexit 0\n");
        std::env::set_var("MEMHG_TEST_SECURITY", security.to_string_lossy().as_ref());
        let script = mount_dir.join("mount_smbfs.sh");
        std::fs::write(
            &script,
            format!(
                "#!/bin/sh\necho \"$@\" > \"{}\"\nfor last in \"$@\"; do mount_point=\"$last\"; done\nmkdir -p \"$mount_point\"\necho 1 > \"$mount_point/.memhg_test_mounted\"\n",
                args_log.display()
            ),
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        std::env::set_var("MEMHG_TEST_MOUNT_SMBFS", script.to_string_lossy().as_ref());
        let library = LibraryService::with_mount_mode(catalog.pool().clone(), mount_dir, true);
        library
            .mount_smb_for_browse(&SmbConnectRequest {
                host: "nas".into(),
                share: "photos".into(),
                username: "guest".into(),
                password: "secret".into(),
                domain: None,
                poll_secs: None,
                sub_path: None,
                read_only: false,
            })
            .await
            .unwrap();
        let args = std::fs::read_to_string(args_log).unwrap();
        assert!(args.contains("-o"));
        assert!(args.contains("ro"));
        std::env::remove_var("MEMHG_TEST_MOUNT_SMBFS");
        std::env::remove_var("MEMHG_TEST_SECURITY");
    }

    #[tokio::test]
    async fn mount_smb_for_browse_returns_mount_path() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let mount_dir = dir.path().join("mounts");
        let script = mount_dir.join("mount_smbfs.sh");
        std::fs::create_dir_all(&mount_dir).unwrap();
        std::fs::write(
            &script,
            "#!/bin/sh\nmkdir -p \"$2\"\necho 1 > \"$2/.memhg_test_mounted\"\n",
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        std::env::set_var("MEMHG_TEST_MOUNT_SMBFS", script.to_string_lossy().as_ref());
        let library = LibraryService::new(catalog.pool().clone(), mount_dir);
        let path = library
            .mount_smb_for_browse(&SmbConnectRequest {
                host: "nas".into(),
                share: "photos".into(),
                username: "guest".into(),
                password: "secret".into(),
                domain: None,
                poll_secs: None,
                sub_path: None,
                read_only: false,
            })
            .await
            .unwrap();
        assert!(path.contains("_browse"));
        std::env::remove_var("MEMHG_TEST_MOUNT_SMBFS");
    }

    #[tokio::test]
    async fn add_smb_source_mounted_registers_existing_path() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let photos = dir.path().join("mounted");
        std::fs::create_dir_all(&photos).unwrap();
        let library = LibraryService::new(catalog.pool().clone(), dir.path().join("mounts"));
        let root = library
            .add_smb_source(SmbSourceInput::Mounted {
                path: photos.to_str().unwrap().into(),
                poll_secs: Some(45),
            })
            .await
            .unwrap();
        assert_eq!(root.kind, "smb");
        assert_eq!(root.poll_secs, Some(45));
    }

    #[tokio::test]
    async fn ensure_smb_mounts_ready_handles_spawn_and_mount_errors() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let repo = SourceRootRepo::new(catalog.pool().clone());
        let mount_dir = dir.path().join("mounts");
        let host = format!("spawn-fail-{}", std::process::id());
        crate::smb::store_credentials(&host, "photos", "user", "secret").unwrap();
        let root = repo
            .insert_smb_mount_root("/missing/library", Some(300), &host, "photos", "user")
            .await
            .unwrap();
        let library = LibraryService::new(catalog.pool().clone(), mount_dir.clone());
        library.ensure_smb_mounts_ready().await.unwrap();
        let updated = repo.get_root(root.id).await.unwrap();
        assert_eq!(updated.status, "offline");

        let host = format!("relink-ready-{}", std::process::id());
        let share_mount = share_mount_point(&mount_dir, &host, "photos", "user");
        let library_dir = share_mount.join("library");
        std::fs::create_dir_all(&library_dir).unwrap();
        std::fs::write(share_mount.join(crate::smb::TEST_MOUNT_MARKER), b"1").unwrap();
        crate::smb::store_credentials(&host, "photos", "user", "secret").unwrap();
        let root = repo
            .insert_smb_mount_root(
                library_dir.to_string_lossy().as_ref(),
                Some(300),
                &host,
                "photos",
                "user",
            )
            .await
            .unwrap();
        library.ensure_smb_mounts_ready().await.unwrap();
        let updated = repo.get_root(root.id).await.unwrap();
        assert_eq!(updated.status, "idle");
    }

    #[tokio::test]
    async fn remove_root_unmounts_when_share_mount_exists() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let mount_dir = dir.path().join("mounts");
        let host = format!("unmount-{}", std::process::id());
        let share_mount = share_mount_point(&mount_dir, &host, "photos", "user");
        std::fs::create_dir_all(&share_mount).unwrap();
        std::fs::write(share_mount.join(crate::smb::TEST_MOUNT_MARKER), b"1").unwrap();
        crate::smb::store_credentials(&host, "photos", "user", "secret").unwrap();
        let repo = SourceRootRepo::new(catalog.pool().clone());
        let root = repo
            .insert_smb_mount_root(
                share_mount.to_string_lossy().as_ref(),
                Some(300),
                &host,
                "photos",
                "user",
            )
            .await
            .unwrap();
        let library = LibraryService::new(catalog.pool().clone(), mount_dir);
        library.remove_root(root.id).await.unwrap();
        assert!(library.list_roots().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn ensure_smb_mounts_ready_skips_incomplete_smb_metadata() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let repo = SourceRootRepo::new(catalog.pool().clone());
        sqlx::query(
            "INSERT INTO source_root (path, kind, scan_policy, status, smb_mounted) VALUES (?, 'smb', 'poll', 'idle', 1)",
        )
        .bind("/orphan")
        .execute(catalog.pool())
        .await
        .unwrap();
        let library = LibraryService::new(catalog.pool().clone(), dir.path().join("mounts"));
        library.ensure_smb_mounts_ready().await.unwrap();
        let roots = repo.list_roots().await.unwrap();
        assert_eq!(roots.len(), 1);
    }

    #[tokio::test]
    async fn relink_root_rejects_non_directory_path() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let library = LibraryService::new(catalog.pool().clone(), dir.path().to_path_buf());
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        let root = library
            .add_local_root(photos.to_str().unwrap())
            .await
            .unwrap();
        let file = dir.path().join("file.txt");
        std::fs::write(&file, b"x").unwrap();
        assert!(library
            .relink_root(root.id, file.to_str().unwrap())
            .await
            .is_err());
    }

    #[tokio::test]
    async fn ensure_smb_mounts_ready_skips_local_roots() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let library = LibraryService::new(catalog.pool().clone(), dir.path().join("mounts"));
        library
            .add_local_root(dir.path().to_str().unwrap())
            .await
            .unwrap();
        library.ensure_smb_mounts_ready().await.unwrap();
    }

    #[tokio::test]
    async fn ensure_smb_mounts_ready_mounts_uncached_share() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let repo = SourceRootRepo::new(catalog.pool().clone());
        let mount_dir = dir.path().join("mounts");
        std::fs::create_dir_all(&mount_dir).unwrap();
        let script = mount_dir.join("mount_smbfs.sh");
        std::fs::write(
            &script,
            "#!/bin/sh\nmkdir -p \"$2\"\necho 1 > \"$2/.memhg_test_mounted\"\n",
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        std::env::set_var("MEMHG_TEST_MOUNT_SMBFS", script.to_string_lossy().as_ref());
        let host = format!("fresh-mount-{}", std::process::id());
        crate::smb::store_credentials(&host, "photos", "user", "secret").unwrap();
        let share_mount = share_mount_point(&mount_dir, &host, "photos", "user");
        let library_dir = share_mount.join("library");
        std::fs::create_dir_all(&library_dir).unwrap();
        let root = repo
            .insert_smb_mount_root(
                library_dir.to_string_lossy().as_ref(),
                Some(300),
                &host,
                "photos",
                "user",
            )
            .await
            .unwrap();
        let library = LibraryService::new(catalog.pool().clone(), mount_dir);
        library.ensure_smb_mounts_ready().await.unwrap();
        let updated = repo.get_root(root.id).await.unwrap();
        assert_eq!(updated.status, "idle");
        assert!(share_mount.join(crate::smb::TEST_MOUNT_MARKER).is_file());
        std::env::remove_var("MEMHG_TEST_MOUNT_SMBFS");
    }

    #[tokio::test]
    async fn relink_smb_root_if_needed_updates_divergent_path() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let repo = SourceRootRepo::new(catalog.pool().clone());
        let mount_dir = dir.path().join("mounts");
        let share_mount = share_mount_point(&mount_dir, "nas", "photos", "user");
        let library_dir = share_mount.join("library");
        std::fs::create_dir_all(&library_dir).unwrap();
        let root = repo
            .insert_smb_mount_root(
                share_mount.to_string_lossy().as_ref(),
                Some(300),
                "nas",
                "photos",
                "user",
            )
            .await
            .unwrap();
        relink_smb_root_if_needed(
            &repo,
            root.id,
            &share_mount,
            "library",
            share_mount.to_str().unwrap(),
        )
        .await
        .unwrap();
        let updated = repo.get_root(root.id).await.unwrap();
        assert_eq!(
            updated.path,
            library_dir.canonicalize().unwrap().to_string_lossy()
        );
    }

    #[tokio::test]
    async fn relink_smb_root_if_needed_noops_when_subfolder_invalid() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let repo = SourceRootRepo::new(catalog.pool().clone());
        let mount_dir = dir.path().join("mounts");
        let share_mount = share_mount_point(&mount_dir, "nas", "photos", "user");
        std::fs::create_dir_all(&share_mount).unwrap();
        let root = repo
            .insert_smb_mount_root(
                share_mount.to_string_lossy().as_ref(),
                Some(300),
                "nas",
                "photos",
                "user",
            )
            .await
            .unwrap();
        relink_smb_root_if_needed(
            &repo,
            root.id,
            &share_mount,
            "../escape",
            share_mount.to_str().unwrap(),
        )
        .await
        .unwrap();
        let updated = repo.get_root(root.id).await.unwrap();
        assert_eq!(updated.path, share_mount.to_string_lossy());
    }

    #[tokio::test]
    async fn relink_smb_root_if_needed_skips_matching_path() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let repo = SourceRootRepo::new(catalog.pool().clone());
        let mount_dir = dir.path().join("mounts");
        let share_mount = share_mount_point(&mount_dir, "nas", "photos", "user");
        let library_dir = share_mount.join("library");
        std::fs::create_dir_all(&library_dir).unwrap();
        let library_path = library_dir
            .canonicalize()
            .unwrap()
            .to_string_lossy()
            .to_string();
        let root = repo
            .insert_smb_mount_root(library_path.as_str(), Some(300), "nas", "photos", "user")
            .await
            .unwrap();
        relink_smb_root_if_needed(&repo, root.id, &share_mount, "library", &library_path)
            .await
            .unwrap();
        let updated = repo.get_root(root.id).await.unwrap();
        assert_eq!(updated.path, library_path);
    }

    #[tokio::test]
    async fn ensure_smb_mounts_ready_relinks_stale_library_path() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let repo = SourceRootRepo::new(catalog.pool().clone());
        let mount_dir = dir.path().join("mounts");
        std::fs::create_dir_all(&mount_dir).unwrap();
        let host = format!("relink-path-{}", std::process::id());
        crate::smb::store_credentials(&host, "photos", "user", "secret").unwrap();
        let share_mount = share_mount_point(&mount_dir, &host, "photos", "user");
        let library_dir = share_mount.join("library/nested");
        std::fs::create_dir_all(&library_dir).unwrap();
        std::fs::write(share_mount.join(crate::smb::TEST_MOUNT_MARKER), b"1").unwrap();
        let link = share_mount.join("linked");
        std::os::unix::fs::symlink(&library_dir, &link).unwrap();
        let root = repo
            .insert_smb_mount_root(
                link.to_string_lossy().as_ref(),
                Some(300),
                &host,
                "photos",
                "user",
            )
            .await
            .unwrap();
        let library = LibraryService::new(catalog.pool().clone(), mount_dir);
        library.ensure_smb_mounts_ready().await.unwrap();
        let updated = repo.get_root(root.id).await.unwrap();
        assert_eq!(
            updated.path,
            library_dir.canonicalize().unwrap().to_string_lossy()
        );
    }

    #[tokio::test]
    async fn ensure_smb_mounts_ready_handles_mount_task_panic() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let repo = SourceRootRepo::new(catalog.pool().clone());
        let host = format!("panic-mount-{}", std::process::id());
        crate::smb::store_credentials(&host, "photos", "user", "secret").unwrap();
        let root = repo
            .insert_smb_mount_root("/missing/panic/library", Some(300), &host, "photos", "user")
            .await
            .unwrap();
        std::env::set_var("MEMHG_TEST_MOUNT_PANIC", "1");
        let library = LibraryService::new(catalog.pool().clone(), dir.path().join("mounts"));
        library.ensure_smb_mounts_ready().await.unwrap();
        let updated = repo.get_root(root.id).await.unwrap();
        assert_eq!(updated.status, "offline");
        std::env::remove_var("MEMHG_TEST_MOUNT_PANIC");
    }

    #[tokio::test]
    async fn preview_relink_rejects_missing_new_path() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let library = LibraryService::new(catalog.pool().clone(), dir.path().to_path_buf());
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        let root = library
            .add_local_root(photos.to_str().unwrap())
            .await
            .unwrap();
        assert!(library
            .preview_relink(root.id, "/missing/relink/path")
            .await
            .is_err());
    }

    #[tokio::test]
    async fn connect_smb_share_rejects_missing_subfolder() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let mount_dir = dir.path().join("mounts");
        let host = format!("subpath-{}", std::process::id());
        let share_mount = share_mount_point(&mount_dir, &host, "photos", "guest");
        std::fs::create_dir_all(&share_mount).unwrap();
        std::fs::write(share_mount.join(crate::smb::TEST_MOUNT_MARKER), b"1").unwrap();
        let library = LibraryService::new(catalog.pool().clone(), mount_dir);
        let err = library
            .connect_smb_share(&SmbConnectRequest {
                host,
                share: "photos".into(),
                username: "guest".into(),
                password: "secret".into(),
                domain: None,
                poll_secs: Some(60),
                sub_path: Some("missing/nested".into()),
                read_only: false,
            })
            .await
            .unwrap_err();
        assert!(err.to_string().contains("folder not found"));
    }

    #[tokio::test]
    async fn mount_smb_for_browse_propagates_mount_task_panic() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        std::env::set_var("MEMHG_TEST_MOUNT_PANIC", "1");
        let library = LibraryService::new(catalog.pool().clone(), dir.path().join("mounts"));
        let err = library
            .mount_smb_for_browse(&SmbConnectRequest {
                host: "nas".into(),
                share: "photos".into(),
                username: "guest".into(),
                password: "secret".into(),
                domain: None,
                poll_secs: None,
                sub_path: None,
                read_only: false,
            })
            .await
            .unwrap_err();
        assert!(err.to_string().contains("SMB mount failed"));
        std::env::remove_var("MEMHG_TEST_MOUNT_PANIC");
    }

    #[tokio::test]
    async fn connect_smb_share_propagates_mount_task_panic() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        std::env::set_var("MEMHG_TEST_MOUNT_PANIC", "1");
        let library = LibraryService::new(catalog.pool().clone(), dir.path().join("mounts"));
        let err = library
            .connect_smb_share(&SmbConnectRequest {
                host: "nas".into(),
                share: "photos".into(),
                username: "guest".into(),
                password: "secret".into(),
                domain: None,
                poll_secs: Some(60),
                sub_path: None,
                read_only: false,
            })
            .await
            .unwrap_err();
        assert!(err.to_string().contains("SMB mount failed"));
        std::env::remove_var("MEMHG_TEST_MOUNT_PANIC");
    }

    #[tokio::test]
    async fn list_folder_children_propagates_blocking_panic() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        std::env::set_var("MEMHG_TEST_LIST_DIRS_PANIC", "1");
        let library = LibraryService::new(catalog.pool().clone(), dir.path().to_path_buf());
        let err = library
            .list_folder_children(dir.path().to_str().unwrap())
            .await
            .unwrap_err();
        assert!(err.to_string().contains("folder listing failed"));
        std::env::remove_var("MEMHG_TEST_LIST_DIRS_PANIC");
    }

    #[tokio::test]
    async fn preview_relink_counts_assets_only_in_old_path() {
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        std::fs::write(
            photos.join("a.jpg"),
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let library = LibraryService::new(catalog.pool().clone(), dir.path().to_path_buf());
        let root = library
            .add_local_root(photos.to_str().unwrap())
            .await
            .unwrap();
        ScanService::new(catalog.pool().clone(), dir.path().join("thumbs"))
            .scan_root(root.id, &ScanControl::noop())
            .await
            .unwrap();
        let moved = dir.path().join("moved");
        std::fs::create_dir_all(&moved).unwrap();
        let preview = library
            .preview_relink(root.id, moved.to_str().unwrap())
            .await
            .unwrap();
        assert_eq!(preview.matched, 1);
        assert_eq!(preview.total_sampled, 1);
    }

    #[tokio::test]
    async fn add_smb_root_rejects_missing_path() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let library = LibraryService::new(catalog.pool().clone(), dir.path().join("mounts"));
        let err = library
            .add_smb_root("/missing/smb/root", None)
            .await
            .unwrap_err();
        assert!(err.to_string().contains("path not found"));
    }

    #[tokio::test]
    async fn preview_relink_samples_multiple_paths() {
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        for name in ["a.jpg", "b.jpg", "c.jpg"] {
            std::fs::write(
                photos.join(name),
                include_bytes!("../../tests/fixtures/minimal.jpg"),
            )
            .unwrap();
        }
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let library = LibraryService::new(catalog.pool().clone(), dir.path().to_path_buf());
        let root = library
            .add_local_root(photos.to_str().unwrap())
            .await
            .unwrap();
        ScanService::new(catalog.pool().clone(), dir.path().join("thumbs"))
            .scan_root(root.id, &ScanControl::noop())
            .await
            .unwrap();
        let preview = library
            .preview_relink(root.id, photos.to_str().unwrap())
            .await
            .unwrap();
        assert_eq!(preview.matched, 3);
        assert_eq!(preview.total_sampled, 3);
    }

    #[tokio::test]
    async fn remove_root_unmounts_existing_share_mount() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let mount_dir = dir.path().join("mounts");
        let host = format!("unmount-existing-{}", std::process::id());
        let share_mount = share_mount_point(&mount_dir, &host, "photos", "user");
        std::fs::create_dir_all(&share_mount).unwrap();
        std::fs::write(share_mount.join(crate::smb::TEST_MOUNT_MARKER), b"1").unwrap();
        crate::smb::store_credentials(&host, "photos", "user", "secret").unwrap();
        let repo = SourceRootRepo::new(catalog.pool().clone());
        let root = repo
            .insert_smb_mount_root(
                share_mount.to_string_lossy().as_ref(),
                Some(300),
                &host,
                "photos",
                "user",
            )
            .await
            .unwrap();
        let library = LibraryService::new(catalog.pool().clone(), mount_dir);
        library.remove_root(root.id).await.unwrap();
        assert!(library.list_roots().await.unwrap().is_empty());
    }
}
