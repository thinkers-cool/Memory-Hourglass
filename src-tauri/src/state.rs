use crate::activity::ActivityRecorder;
use crate::catalog::Catalog;
use crate::collection::CollectionRepo;
use crate::error::{AppError, Result};
use crate::export::ExportService;
use crate::jobs::JobQueue;
use crate::library::LibraryService;
use crate::link::LinkService;
use crate::query::QueryService;
use crate::scan::ScanService;
use crate::watcher::WatcherService;
use crate::workspace::{
    workspace_mounts_dir, WorkspaceInfo, WorkspaceMediaSettings, WorkspacePaths, WorkspaceService,
};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::{watch, RwLock};

pub struct ScanStatus {
    pub root_id: Option<i64>,
    pub stage: String,
    pub scanned: u64,
    pub indexed: u64,
    pub running: bool,
}

pub struct ActiveWorkspace {
    pub info: WorkspaceInfo,
    pub paths: WorkspacePaths,
    pub media_settings: WorkspaceMediaSettings,
    pub catalog: Catalog,
    pub thumb_dir: PathBuf,
    pub library: LibraryService,
    pub jobs: JobQueue,
    pub scan_status: Arc<RwLock<ScanStatus>>,
    pub collection: CollectionRepo,
    pub activity: ActivityRecorder,
    pub link: LinkService,
    pub shutdown_tx: watch::Sender<bool>,
    pub scan_pause: Arc<AtomicBool>,
    pub roots_refresh: watch::Sender<u64>,
}

impl ActiveWorkspace {
    pub async fn open(path: &Path, app_data_dir: &Path) -> Result<Self> {
        let canonical = std::fs::canonicalize(path).map_err(|_| {
            AppError::Workspace(format!("workspace path not found: {}", path.display()))
        })?;
        let info = crate::workspace::workspace_info(&canonical)?;
        let paths = WorkspacePaths::new(canonical);
        crate::workspace::ensure_workspace_dirs(&paths, info.read_only)?;
        let media_settings = WorkspaceMediaSettings::from_paths(&paths, info.read_only);

        let catalog = Catalog::open(&paths.catalog_path()).await?;
        let pool = catalog.pool().clone();
        let thumb_dir = paths.thumbs_dir();
        let mount_dir = workspace_mounts_dir(app_data_dir, &info.id);
        std::fs::create_dir_all(&mount_dir)?;

        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        let (roots_refresh, roots_refresh_rx) = watch::channel(0u64);

        let watcher = WatcherService::new(pool.clone(), thumb_dir.clone(), media_settings.clone());
        let jobs = JobQueue::new();
        watcher.spawn_dynamic_local_watcher(jobs.clone(), shutdown_rx.clone(), roots_refresh_rx);
        watcher.spawn_smb_poller(jobs.clone(), shutdown_rx);

        let library = LibraryService::with_mount_mode(pool.clone(), mount_dir, info.read_only);
        library.ensure_smb_mounts_ready().await?;

        Ok(Self {
            info,
            paths,
            media_settings,
            catalog,
            thumb_dir,
            library,
            jobs,
            scan_status: Arc::new(RwLock::new(ScanStatus {
                root_id: None,
                stage: "idle".into(),
                scanned: 0,
                indexed: 0,
                running: false,
            })),
            collection: CollectionRepo::new(pool.clone()),
            activity: ActivityRecorder::new(pool.clone()),
            link: LinkService::new(pool.clone()),
            shutdown_tx,
            scan_pause: Arc::new(AtomicBool::new(false)),
            roots_refresh,
        })
    }

    pub fn notify_roots_refresh(&self) {
        let next = *self.roots_refresh.borrow() + 1;
        let _ = self.roots_refresh.send(next);
    }

    pub fn scan_control(&self) -> crate::scan::ScanControl {
        crate::scan::ScanControl::new(self.scan_pause.clone(), self.jobs.cancel_flag())
    }

    pub fn pause_scan(&self) {
        self.scan_pause.store(true, Ordering::SeqCst);
    }

    pub fn resume_scan(&self) {
        self.scan_pause.store(false, Ordering::SeqCst);
    }

    pub fn scan_service(&self) -> ScanService {
        ScanService::with_media_settings(
            self.catalog.pool().clone(),
            self.thumb_dir.clone(),
            self.media_settings.clone(),
        )
    }

    pub fn query_service(&self) -> QueryService {
        QueryService::with_media_settings(
            self.catalog.pool().clone(),
            self.thumb_dir.clone(),
            self.media_settings.clone(),
        )
    }

    pub fn export_service(&self) -> ExportService {
        ExportService::new(self.catalog.pool().clone())
    }

    pub fn shutdown(&self) {
        let _ = self.shutdown_tx.send(true);
    }
}

pub struct AppState {
    pub workspaces: RwLock<WorkspaceService>,
    pub active: RwLock<Option<Arc<ActiveWorkspace>>>,
}

impl AppState {
    pub fn new(app_data_dir: PathBuf) -> Result<Self> {
        let workspaces = WorkspaceService::load(app_data_dir)?;
        Ok(Self {
            workspaces: RwLock::new(workspaces),
            active: RwLock::new(None),
        })
    }

    pub async fn active_workspace(&self) -> Result<Arc<ActiveWorkspace>> {
        let guard = self.active.read().await;
        guard
            .clone()
            .ok_or_else(|| AppError::Workspace("no workspace open".into()))
    }

    pub async fn with_active<F, Fut, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(Arc<ActiveWorkspace>) -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let ws = self.active_workspace().await?;
        f(ws).await
    }

    pub async fn open_workspace(&self, path: &Path) -> Result<WorkspaceInfo> {
        let info = {
            let workspaces = self.workspaces.read().await;
            workspaces.validate_open(path)?
        };

        self.close_workspace().await?;

        let app_data_dir = {
            let workspaces = self.workspaces.read().await;
            workspaces.app_data_dir().to_path_buf()
        };
        let session = ActiveWorkspace::open(path, &app_data_dir).await?;
        *self.active.write().await = Some(Arc::new(session));

        let mut workspaces = self.workspaces.write().await;
        workspaces.touch_opened(&info)?;

        Ok(info)
    }

    pub async fn close_workspace(&self) -> Result<()> {
        let mut guard = self.active.write().await;
        if let Some(ws) = guard.take() {
            ws.shutdown();
        }
        Ok(())
    }

    pub async fn active_workspace_info(&self) -> Option<WorkspaceInfo> {
        let guard = self.active.read().await;
        guard.as_ref().map(|ws| ws.info.clone())
    }

    pub async fn try_open_last_workspace(&self) -> Result<Option<WorkspaceInfo>> {
        let last = {
            let workspaces = self.workspaces.read().await;
            workspaces.last_opened()
        };
        let Some(path) = last else {
            return Ok(None);
        };
        let path_buf = PathBuf::from(&path);
        if !crate::workspace::is_workspace_folder(&path_buf) {
            let mut workspaces = self.workspaces.write().await;
            workspaces.remove_recent(&path)?;
            return Ok(None);
        }
        let info = self.open_workspace(&path_buf).await?;
        Ok(Some(info))
    }
}

#[cfg(test)]
impl AppState {
    pub async fn test_with_workspace(workspace_root: &Path) -> Result<Self> {
        let app_data = workspace_root
            .parent()
            .unwrap_or(workspace_root)
            .join("_app_data");
        let state = AppState::new(app_data)?;
        if !crate::workspace::is_workspace_folder(workspace_root) {
            std::fs::create_dir_all(workspace_root)?;
            crate::workspace::init_workspace_at(workspace_root, false)?;
        }
        state.open_workspace(workspace_root).await?;
        Ok(state)
    }

    pub async fn test_with_fresh_workspace() -> Result<(Self, tempfile::TempDir)> {
        let dir = tempfile::tempdir().unwrap();
        let created = dir.path().join("Test Workspace");
        std::fs::create_dir_all(&created).unwrap();
        crate::workspace::init_workspace_at(&created, false)?;
        let state = AppState::test_with_workspace(&created).await?;
        Ok((state, dir))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::Ordering;
    use tempfile::tempdir;

    #[tokio::test]
    async fn open_and_close_workspace() {
        let dir = tempdir().unwrap();
        let ws_path = dir.path().join("Demo");
        std::fs::create_dir_all(&ws_path).unwrap();
        let info = crate::workspace::init_workspace_at(&ws_path, false).unwrap();
        let app_data = dir.path().join("app");
        let state = AppState::new(app_data).unwrap();

        assert!(state.active_workspace_info().await.is_none());

        let opened = state.open_workspace(Path::new(&info.path)).await.unwrap();
        assert_eq!(opened, info);
        assert_eq!(state.active_workspace_info().await.unwrap().path, info.path);

        state.close_workspace().await.unwrap();
        assert!(state.active_workspace_info().await.is_none());
    }

    #[tokio::test]
    async fn with_active_requires_open_workspace() {
        let dir = tempdir().unwrap();
        let state = AppState::new(dir.path().join("app")).unwrap();
        let err = state.with_active(|_| async { Ok(()) }).await.unwrap_err();
        assert!(err.to_string().contains("no workspace open"));
    }

    #[tokio::test]
    async fn with_active_propagates_callback_error() {
        let (state, _dir) = AppState::test_with_fresh_workspace().await.unwrap();
        let err = state
            .with_active(|_| async {
                Err::<(), AppError>(AppError::Workspace("callback failed".into()))
            })
            .await
            .unwrap_err();
        assert!(err.to_string().contains("callback failed"));
    }

    #[tokio::test]
    async fn pause_and_resume_scan_flags() {
        let (state, _dir) = AppState::test_with_fresh_workspace().await.unwrap();
        state
            .with_active(|ws| async move {
                assert!(!ws.scan_pause.load(Ordering::SeqCst));
                ws.pause_scan();
                assert!(ws.scan_pause.load(Ordering::SeqCst));
                ws.resume_scan();
                assert!(!ws.scan_pause.load(Ordering::SeqCst));
                Ok(())
            })
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn notify_roots_refresh_increments_generation() {
        let (state, _dir) = AppState::test_with_fresh_workspace().await.unwrap();
        state
            .with_active(|ws| async move {
                let mut refresh = ws.roots_refresh.subscribe();
                assert_eq!(*refresh.borrow(), 0);
                ws.notify_roots_refresh();
                refresh.changed().await.unwrap();
                assert_eq!(*refresh.borrow(), 1);
                Ok(())
            })
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn try_open_last_workspace_skips_missing_path() {
        let dir = tempdir().unwrap();
        let app_data = dir.path().join("app");
        let mut service = WorkspaceService::load(app_data.clone()).unwrap();
        service.registry_mut().touch_recent(&WorkspaceInfo {
            path: "/missing/workspace".into(),
            name: "Missing".into(),
            id: "id".into(),
            read_only: false,
        });
        service.save_registry().unwrap();

        let state = AppState::new(app_data).unwrap();
        let opened = state.try_open_last_workspace().await.unwrap();
        assert!(opened.is_none());
    }

    #[tokio::test]
    async fn try_open_last_workspace_opens_valid_recent() {
        let dir = tempdir().unwrap();
        let ws_path = dir.path().join("Recent");
        std::fs::create_dir_all(&ws_path).unwrap();
        let info = crate::workspace::init_workspace_at(&ws_path, false).unwrap();
        let app_data = dir.path().join("app");
        let mut service = WorkspaceService::load(app_data.clone()).unwrap();
        service.touch_opened(&info).unwrap();

        let state = AppState::new(app_data).unwrap();
        let opened = state.try_open_last_workspace().await.unwrap().unwrap();
        assert_eq!(opened.path, info.path);
        assert!(state.active_workspace_info().await.is_some());
    }

    #[tokio::test]
    async fn open_workspace_replaces_previous_session() {
        let dir = tempdir().unwrap();
        let first = dir.path().join("First");
        let second = dir.path().join("Second");
        std::fs::create_dir_all(&first).unwrap();
        std::fs::create_dir_all(&second).unwrap();
        let first_info = crate::workspace::init_workspace_at(&first, false).unwrap();
        let second_info = crate::workspace::init_workspace_at(&second, false).unwrap();
        let state = AppState::new(dir.path().join("app")).unwrap();
        state
            .open_workspace(Path::new(&first_info.path))
            .await
            .unwrap();
        state
            .open_workspace(Path::new(&second_info.path))
            .await
            .unwrap();
        assert_eq!(
            state.active_workspace_info().await.unwrap().path,
            second_info.path
        );
    }

    #[tokio::test]
    async fn active_workspace_exposes_service_factories() {
        let (state, _dir) = AppState::test_with_fresh_workspace().await.unwrap();
        state
            .with_active(|ws| async move {
                let _ = ws.scan_control();
                let _ = ws.scan_service();
                let _ = ws.query_service();
                let _ = ws.export_service();
                Ok(())
            })
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn try_open_last_workspace_returns_none_without_history() {
        let dir = tempdir().unwrap();
        let state = AppState::new(dir.path().join("app")).unwrap();
        assert!(state.try_open_last_workspace().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_with_workspace_initializes_plain_folder() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("Plain");
        let state = AppState::test_with_workspace(&root).await.unwrap();
        assert!(state.active_workspace_info().await.is_some());
        assert!(crate::workspace::is_workspace_folder(&root));
    }

    #[tokio::test]
    async fn open_workspace_succeeds_for_read_only_workspace() {
        let dir = tempdir().unwrap();
        let ws_path = dir.path().join("ReadOnly");
        std::fs::create_dir_all(&ws_path).unwrap();
        let info = crate::workspace::init_workspace_at(&ws_path, true).unwrap();
        let state = AppState::new(dir.path().join("app")).unwrap();
        let opened = state.open_workspace(Path::new(&info.path)).await.unwrap();
        assert!(opened.read_only);
        assert!(state.active_workspace_info().await.unwrap().read_only);
    }

    #[tokio::test]
    async fn open_workspace_rejects_missing_path() {
        let dir = tempdir().unwrap();
        let state = AppState::new(dir.path().join("app")).unwrap();
        let err = state
            .open_workspace(Path::new("/nonexistent/workspace/path"))
            .await
            .unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[tokio::test]
    async fn open_workspace_rejects_non_workspace_folder() {
        let dir = tempdir().unwrap();
        let plain = dir.path().join("plain");
        std::fs::create_dir_all(&plain).unwrap();
        let state = AppState::new(dir.path().join("app")).unwrap();
        let err = state.open_workspace(&plain).await.unwrap_err();
        assert!(!err.to_string().is_empty());
    }

    #[tokio::test]
    async fn active_workspace_requires_open_session() {
        let dir = tempdir().unwrap();
        let state = AppState::new(dir.path().join("app")).unwrap();
        assert!(state.active_workspace().await.is_err());
    }

    #[tokio::test]
    async fn close_workspace_without_active_succeeds() {
        let dir = tempdir().unwrap();
        let state = AppState::new(dir.path().join("app")).unwrap();
        state.close_workspace().await.unwrap();
        assert!(state.active_workspace_info().await.is_none());
    }

    #[tokio::test]
    async fn active_workspace_open_rejects_missing_path_directly() {
        let dir = tempdir().unwrap();
        let result =
            ActiveWorkspace::open(&dir.path().join("missing"), &dir.path().join("app")).await;
        assert!(result.is_err());
        assert!(result.err().unwrap().to_string().contains("not found"));
    }

    #[tokio::test]
    async fn active_workspace_open_rejects_corrupt_catalog_directly() {
        let dir = tempdir().unwrap();
        let ws_path = dir.path().join("BrokenCatalog");
        std::fs::create_dir_all(&ws_path).unwrap();
        crate::workspace::init_workspace_at(&ws_path, false).unwrap();
        std::fs::write(ws_path.join("catalog.db"), b"broken").unwrap();
        let err = ActiveWorkspace::open(&ws_path, &dir.path().join("app")).await;
        assert!(err.is_err());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn active_workspace_open_rejects_unwritable_mount_dir() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempdir().unwrap();
        let ws_path = dir.path().join("MountBlocked");
        std::fs::create_dir_all(&ws_path).unwrap();
        crate::workspace::init_workspace_at(&ws_path, false).unwrap();
        let app_data = dir.path().join("app-data-file");
        std::fs::write(&app_data, b"x").unwrap();
        std::fs::set_permissions(&app_data, std::fs::Permissions::from_mode(0o000)).unwrap();
        let err = ActiveWorkspace::open(&ws_path, &app_data).await;
        assert!(err.is_err());
        std::fs::set_permissions(&app_data, std::fs::Permissions::from_mode(0o644)).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn app_state_new_fails_when_app_data_is_file() {
        let dir = tempdir().unwrap();
        let blocker = dir.path().join("app-data");
        std::fs::write(&blocker, b"not-a-directory").unwrap();
        assert!(AppState::new(blocker).is_err());
    }

    #[tokio::test]
    async fn try_open_last_workspace_errors_when_catalog_corrupt() {
        let dir = tempdir().unwrap();
        let ws_path = dir.path().join("RecentBroken");
        std::fs::create_dir_all(&ws_path).unwrap();
        let info = crate::workspace::init_workspace_at(&ws_path, false).unwrap();
        let app_data = dir.path().join("app");
        let mut service = WorkspaceService::load(app_data.clone()).unwrap();
        service.touch_opened(&info).unwrap();
        std::fs::write(ws_path.join("catalog.db"), b"broken").unwrap();
        let state = AppState::new(app_data).unwrap();
        assert!(state.try_open_last_workspace().await.is_err());
    }

    #[tokio::test]
    async fn try_open_last_workspace_prunes_invalid_workspace_folder() {
        let dir = tempdir().unwrap();
        let app_data = dir.path().join("app");
        let plain = dir.path().join("plain-folder");
        std::fs::create_dir_all(&plain).unwrap();
        let mut service = WorkspaceService::load(app_data.clone()).unwrap();
        service.registry_mut().touch_recent(&WorkspaceInfo {
            path: plain.to_string_lossy().to_string(),
            name: "Plain".into(),
            id: "plain".into(),
            read_only: false,
        });
        service.save_registry().unwrap();
        let state = AppState::new(app_data.clone()).unwrap();
        assert!(state.try_open_last_workspace().await.unwrap().is_none());
        let registry = WorkspaceService::load(app_data).unwrap();
        assert!(registry.list_recent().is_empty());
    }

    #[tokio::test]
    async fn close_workspace_shuts_down_active_session() {
        let (state, _dir) = AppState::test_with_fresh_workspace().await.unwrap();
        assert!(state.active_workspace_info().await.is_some());
        state.close_workspace().await.unwrap();
        assert!(state.active_workspace_info().await.is_none());
    }

    #[tokio::test]
    async fn with_active_runs_closure_for_open_workspace() {
        let (state, _dir) = AppState::test_with_fresh_workspace().await.unwrap();
        let name = state
            .with_active(|ws| async move { Ok(ws.info.name.clone()) })
            .await
            .unwrap();
        assert!(!name.is_empty());
    }

    #[tokio::test]
    async fn test_with_fresh_workspace_returns_open_state() {
        let (state, _dir) = AppState::test_with_fresh_workspace().await.unwrap();
        let info = state.active_workspace_info().await.unwrap();
        assert!(!info.path.is_empty());
    }

    #[tokio::test]
    async fn open_workspace_validates_before_opening() {
        let dir = tempdir().unwrap();
        let ws_path = dir.path().join("Validated");
        std::fs::create_dir_all(&ws_path).unwrap();
        let info = crate::workspace::init_workspace_at(&ws_path, false).unwrap();
        let state = AppState::new(dir.path().join("app")).unwrap();
        let opened = state.open_workspace(Path::new(&info.path)).await.unwrap();
        assert_eq!(opened.id, info.id);
    }

    #[tokio::test]
    async fn try_open_last_workspace_removes_stale_non_workspace_path() {
        let dir = tempdir().unwrap();
        let app_data = dir.path().join("app");
        let plain = dir.path().join("plain");
        std::fs::create_dir_all(&plain).unwrap();
        let mut service = WorkspaceService::load(app_data.clone()).unwrap();
        service.registry_mut().touch_recent(&WorkspaceInfo {
            path: plain.to_string_lossy().to_string(),
            name: "Plain".into(),
            id: "plain".into(),
            read_only: false,
        });
        service.save_registry().unwrap();
        let state = AppState::new(app_data).unwrap();
        assert!(state.try_open_last_workspace().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn active_workspace_open_ensures_smb_mounts_on_startup() {
        let (state, _dir) = AppState::test_with_fresh_workspace().await.unwrap();
        state
            .with_active(|ws| async move {
                let roots = ws.library.list_roots().await.unwrap();
                assert!(roots.is_empty());
                Ok(())
            })
            .await
            .unwrap();
    }
}
