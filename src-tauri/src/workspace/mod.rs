mod registry;

use crate::error::{AppError, Result};
use chrono::Utc;
use registry::WorkspaceRegistry;
use serde::{Deserialize, Serialize};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::path::{Path, PathBuf};

pub use registry::{RecentWorkspace, WorkspaceRegistryFile};

pub const WORKSPACE_MANIFEST: &str = "workspace.json";
pub const CATALOG_DB: &str = "catalog.db";
pub const THUMBS_DIR: &str = "thumbs";
pub const XMP_DIR: &str = "xmp";
pub const MOUNTS_DIR: &str = "mounts";
pub const SCHEMA_VERSION: u32 = 1;
pub const MAX_RECENT: usize = 10;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceManifest {
    pub id: String,
    pub name: String,
    pub schema_version: u32,
    pub created_at: String,
    #[serde(default)]
    pub read_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceInfo {
    pub path: String,
    pub name: String,
    pub id: String,
    pub read_only: bool,
}

#[derive(Debug, Clone)]
pub struct WorkspaceMediaSettings {
    pub read_only: bool,
    pub workspace_xmp_dir: PathBuf,
}

impl Default for WorkspaceMediaSettings {
    fn default() -> Self {
        Self {
            read_only: false,
            workspace_xmp_dir: PathBuf::new(),
        }
    }
}

impl WorkspaceMediaSettings {
    pub fn from_paths(paths: &WorkspacePaths, read_only: bool) -> Self {
        Self {
            read_only,
            workspace_xmp_dir: if read_only {
                paths.xmp_dir()
            } else {
                PathBuf::new()
            },
        }
    }
}

pub struct WorkspacePaths {
    pub root: PathBuf,
}

impl WorkspacePaths {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn manifest_path(&self) -> PathBuf {
        self.root.join(WORKSPACE_MANIFEST)
    }

    pub fn catalog_path(&self) -> PathBuf {
        self.root.join(CATALOG_DB)
    }

    pub fn thumbs_dir(&self) -> PathBuf {
        self.root.join(THUMBS_DIR)
    }

    pub fn xmp_dir(&self) -> PathBuf {
        self.root.join(XMP_DIR)
    }
}

pub fn workspace_mounts_dir(app_data_dir: &Path, workspace_id: &str) -> PathBuf {
    app_data_dir.join(MOUNTS_DIR).join(workspace_id)
}

pub fn workspace_display_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .map(|name| name.to_string())
        .unwrap_or_else(|| "workspace".to_string())
}

fn directory_is_empty(path: &Path) -> Result<bool> {
    if !path.exists() {
        return Ok(true);
    }
    Ok(std::fs::read_dir(path)?.next().is_none())
}

pub fn init_workspace_at(path: &Path, read_only: bool) -> Result<WorkspaceInfo> {
    if !path.exists() {
        return Err(AppError::Workspace(format!(
            "workspace path not found: {}",
            path.display()
        )));
    }

    let canonical = std::fs::canonicalize(path).map_err(|_| {
        AppError::Workspace(format!("workspace path not found: {}", path.display()))
    })?;
    if !canonical.is_dir() {
        return Err(AppError::Workspace(format!(
            "workspace path is not a directory: {}",
            canonical.display()
        )));
    }

    if is_workspace_folder(&canonical) {
        return Err(AppError::Workspace(
            "folder is already a workspace; use Open Workspace".into(),
        ));
    }

    if !directory_is_empty(&canonical)? {
        return Err(AppError::Workspace(
            "folder is not empty; choose an empty folder for a new workspace".into(),
        ));
    }

    let paths = WorkspacePaths::new(canonical.clone());
    ensure_workspace_dirs(&paths, read_only)?;

    let manifest = WorkspaceManifest {
        id: format!("ws-{}", Utc::now().timestamp_millis()),
        name: workspace_display_name(&canonical),
        schema_version: SCHEMA_VERSION,
        created_at: Utc::now().to_rfc3339(),
        read_only,
    };
    write_manifest(&paths, &manifest)?;

    workspace_info(&canonical)
}

pub fn is_workspace_folder(path: &Path) -> bool {
    path.join(WORKSPACE_MANIFEST).is_file()
}

pub fn workspace_path_valid(path: &str) -> bool {
    let path = Path::new(path);
    path.is_dir() && is_workspace_folder(path)
}

pub fn read_manifest(path: &Path) -> Result<WorkspaceManifest> {
    let manifest_path = path.join(WORKSPACE_MANIFEST);
    if !manifest_path.is_file() {
        return Err(AppError::Workspace(format!(
            "missing {} in {}",
            WORKSPACE_MANIFEST,
            path.display()
        )));
    }
    let raw = std::fs::read_to_string(&manifest_path)?;
    let manifest: WorkspaceManifest = serde_json::from_str(&raw)
        .map_err(|e| AppError::Workspace(format!("invalid {}: {}", WORKSPACE_MANIFEST, e)))?;
    if manifest.schema_version != SCHEMA_VERSION {
        return Err(AppError::Workspace(format!(
            "unsupported workspace schema version {}",
            manifest.schema_version
        )));
    }
    Ok(manifest)
}

pub fn workspace_info(path: &Path) -> Result<WorkspaceInfo> {
    let canonical = std::fs::canonicalize(path).map_err(|_| {
        AppError::Workspace(format!("workspace path not found: {}", path.display()))
    })?;
    let manifest = read_manifest(&canonical)?;
    Ok(WorkspaceInfo {
        path: canonical.to_string_lossy().to_string(),
        name: manifest.name,
        id: manifest.id,
        read_only: manifest.read_only,
    })
}

pub fn ensure_workspace_dirs(paths: &WorkspacePaths, read_only: bool) -> Result<()> {
    std::fs::create_dir_all(&paths.root)?;
    std::fs::create_dir_all(paths.thumbs_dir())?;
    if read_only {
        std::fs::create_dir_all(paths.xmp_dir())?;
    }
    Ok(())
}

pub fn write_manifest(paths: &WorkspacePaths, manifest: &WorkspaceManifest) -> Result<()> {
    let json = serde_json::to_string_pretty(manifest)?;
    std::fs::write(paths.manifest_path(), json)?;
    Ok(())
}

pub fn create_workspace(path: &Path, read_only: bool) -> Result<WorkspaceInfo> {
    init_workspace_at(path, read_only)
}

pub struct WorkspaceService {
    app_data_dir: PathBuf,
    registry: WorkspaceRegistry,
}

impl WorkspaceService {
    pub fn load(app_data_dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&app_data_dir)?;
        let registry = WorkspaceRegistry::load(&app_data_dir)?;
        Ok(Self {
            app_data_dir,
            registry,
        })
    }

    pub fn app_data_dir(&self) -> &Path {
        &self.app_data_dir
    }

    pub fn registry(&self) -> &WorkspaceRegistry {
        &self.registry
    }

    pub fn registry_mut(&mut self) -> &mut WorkspaceRegistry {
        &mut self.registry
    }

    pub fn save_registry(&self) -> Result<()> {
        self.registry.save(&self.app_data_dir)
    }

    pub fn create(&mut self, path: &Path, read_only: bool) -> Result<WorkspaceInfo> {
        let info = create_workspace(path, read_only)?;
        self.registry.touch_recent(&info);
        self.save_registry()?;
        Ok(info)
    }

    pub fn validate_open(&self, path: &Path) -> Result<WorkspaceInfo> {
        let canonical = std::fs::canonicalize(path).map_err(|_| {
            AppError::Workspace(format!("workspace path not found: {}", path.display()))
        })?;
        if !canonical.is_dir() {
            return Err(AppError::Workspace(format!(
                "workspace path is not a directory: {}",
                canonical.display()
            )));
        }
        if !is_workspace_folder(&canonical) {
            return Err(AppError::Workspace(format!(
                "not a workspace; missing {} in {}",
                WORKSPACE_MANIFEST,
                canonical.display()
            )));
        }
        workspace_info(&canonical)
    }

    pub fn touch_opened(&mut self, info: &WorkspaceInfo) -> Result<()> {
        self.registry.touch_recent(info);
        self.save_registry()?;
        Ok(())
    }

    pub fn remove_recent(&mut self, path: &str) -> Result<()> {
        self.registry.remove_recent(path);
        self.save_registry()?;
        Ok(())
    }

    pub fn list_recent(&self) -> Vec<RecentWorkspace> {
        self.registry
            .list_recent()
            .into_iter()
            .map(|entry| {
                let valid = workspace_path_valid(&entry.path);
                RecentWorkspace {
                    path: entry.path,
                    name: entry.name,
                    last_opened: entry.last_opened,
                    valid,
                    root_count: 0,
                    album_count: 0,
                    tag_count: 0,
                    read_only: entry.read_only,
                }
            })
            .collect()
    }

    pub async fn enrich_recent_entry(entry: RecentWorkspace) -> RecentWorkspace {
        if !entry.valid {
            return entry;
        }
        let path = Path::new(&entry.path);
        let counts = read_workspace_summary_counts(path).await;
        let read_only = read_manifest(path)
            .map(|manifest| manifest.read_only)
            .unwrap_or(entry.read_only);
        RecentWorkspace {
            root_count: counts.root_count,
            album_count: counts.album_count,
            tag_count: counts.tag_count,
            read_only,
            ..entry
        }
    }

    pub fn last_opened(&self) -> Option<String> {
        self.registry.last_opened()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WorkspaceSummaryCounts {
    pub root_count: u32,
    pub album_count: u32,
    pub tag_count: u32,
}

pub async fn read_workspace_summary_counts(path: &Path) -> WorkspaceSummaryCounts {
    let empty = WorkspaceSummaryCounts::default();
    if !is_workspace_folder(path) {
        return empty;
    }

    let catalog_path = path.join(CATALOG_DB);
    if !catalog_path.is_file() {
        return empty;
    }

    let options = SqliteConnectOptions::new()
        .filename(&catalog_path)
        .read_only(true);
    let pool = match SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
    {
        Ok(pool) => pool,
        Err(_) => return empty,
    };

    let root_count = count_table(&pool, WorkspaceCountTable::Roots).await;
    let album_count = count_table(&pool, WorkspaceCountTable::Albums).await;
    let tag_count = count_table(&pool, WorkspaceCountTable::Tags).await;
    pool.close().await;

    WorkspaceSummaryCounts {
        root_count,
        album_count,
        tag_count,
    }
}

async fn count_table(pool: &sqlx::sqlite::SqlitePool, table: WorkspaceCountTable) -> u32 {
    let query = match table {
        WorkspaceCountTable::Roots => "SELECT COUNT(*) FROM source_root",
        WorkspaceCountTable::Albums => "SELECT COUNT(*) FROM album",
        WorkspaceCountTable::Tags => "SELECT COUNT(*) FROM tag",
    };
    sqlx::query_scalar::<_, i64>(query)
        .fetch_one(pool)
        .await
        .unwrap_or(0)
        .max(0) as u32
}

enum WorkspaceCountTable {
    Roots,
    Albums,
    Tags,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn workspace_display_name_uses_folder_name() {
        assert_eq!(
            workspace_display_name(Path::new("/projects/Wedding 2024")),
            "Wedding 2024"
        );
        assert_eq!(workspace_display_name(Path::new("/projects")), "projects");
    }

    #[test]
    fn init_workspace_writes_manifest_and_dirs() {
        let dir = tempdir().unwrap();
        let workspace_root = dir.path().join("Family Archive");
        std::fs::create_dir_all(&workspace_root).unwrap();
        let info = init_workspace_at(&workspace_root, false).unwrap();
        assert!(workspace_root.join(WORKSPACE_MANIFEST).is_file());
        assert!(workspace_root.join(THUMBS_DIR).is_dir());
        assert_eq!(info.name, "Family Archive");
        assert!(!info.read_only);
        assert!(is_workspace_folder(&workspace_root));
    }

    #[test]
    fn init_read_only_workspace_creates_xmp_dir() {
        let dir = tempdir().unwrap();
        let workspace_root = dir.path().join("Archive");
        std::fs::create_dir_all(&workspace_root).unwrap();
        let info = init_workspace_at(&workspace_root, true).unwrap();
        assert!(info.read_only);
        assert!(workspace_root.join(XMP_DIR).is_dir());
        let manifest = read_manifest(&workspace_root).unwrap();
        assert!(manifest.read_only);
    }

    #[test]
    fn workspace_mounts_dir_lives_under_app_data() {
        let app_data = Path::new("/app");
        let mounts = workspace_mounts_dir(app_data, "ws-123");
        assert_eq!(mounts, PathBuf::from("/app/mounts/ws-123"));
    }

    #[test]
    fn init_workspace_rejects_nonempty_folder() {
        let dir = tempdir().unwrap();
        let existing = dir.path().join("Family Archive");
        std::fs::create_dir_all(&existing).unwrap();
        std::fs::write(existing.join("note.txt"), b"x").unwrap();
        let err = init_workspace_at(&existing, false).unwrap_err();
        assert!(err.to_string().contains("not empty"));
    }

    #[test]
    fn init_workspace_rejects_existing_workspace() {
        let dir = tempdir().unwrap();
        let workspace_root = dir.path().join("Demo");
        std::fs::create_dir_all(&workspace_root).unwrap();
        init_workspace_at(&workspace_root, false).unwrap();
        let err = init_workspace_at(&workspace_root, false).unwrap_err();
        assert!(err.to_string().contains("already a workspace"));
    }

    #[test]
    fn validate_open_rejects_non_workspace_folder() {
        let dir = tempdir().unwrap();
        let folder = dir.path().join("plain");
        std::fs::create_dir_all(&folder).unwrap();
        let app_data = dir.path().join("app");
        let service = WorkspaceService::load(app_data).unwrap();
        let err = service.validate_open(&folder).unwrap_err();
        assert!(err.to_string().contains("not a workspace"));
    }

    #[test]
    fn workspace_info_reads_manifest() {
        let dir = tempdir().unwrap();
        let workspace_root = dir.path().join("Test");
        std::fs::create_dir_all(&workspace_root).unwrap();
        let info = init_workspace_at(&workspace_root, false).unwrap();
        let again = workspace_info(&workspace_root).unwrap();
        assert_eq!(info, again);
    }

    #[test]
    fn read_manifest_rejects_missing_file() {
        let dir = tempdir().unwrap();
        let err = read_manifest(dir.path()).unwrap_err();
        assert!(err.to_string().contains(WORKSPACE_MANIFEST));
    }

    #[test]
    fn registry_persists_recent_workspaces() {
        let dir = tempdir().unwrap();
        let app_data = dir.path().join("app");
        let workspace_root = dir.path().join("One");
        std::fs::create_dir_all(&workspace_root).unwrap();
        let info = init_workspace_at(&workspace_root, false).unwrap();
        let mut service = WorkspaceService::load(app_data.clone()).unwrap();
        service.touch_opened(&info).unwrap();
        let recent = service.list_recent();
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].name, "One");
        assert!(recent[0].valid);

        let reloaded = WorkspaceService::load(app_data).unwrap();
        assert_eq!(reloaded.list_recent().len(), 1);
        assert_eq!(reloaded.list_recent()[0].valid, true);
        assert_eq!(reloaded.last_opened().as_deref(), Some(info.path.as_str()));
    }

    #[test]
    fn list_recent_marks_missing_paths_invalid() {
        let dir = tempdir().unwrap();
        let app_data = dir.path().join("app");
        let mut service = WorkspaceService::load(app_data).unwrap();
        service.registry_mut().touch_recent(&WorkspaceInfo {
            path: "/missing/workspace".into(),
            name: "Missing".into(),
            id: "id".into(),
            read_only: false,
        });
        service.save_registry().unwrap();

        let recent = service.list_recent();
        assert_eq!(recent.len(), 1);
        assert!(!recent[0].valid);
    }

    #[tokio::test]
    async fn read_workspace_summary_counts_reads_catalog_tables() {
        use crate::catalog::Catalog;

        let dir = tempdir().unwrap();
        let workspace_root = dir.path().join("Demo");
        std::fs::create_dir_all(&workspace_root).unwrap();
        init_workspace_at(&workspace_root, false).unwrap();
        let catalog = Catalog::open(&workspace_root.join(CATALOG_DB))
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO source_root (path, kind, scan_policy, status) VALUES (?, 'local', 'manual', 'idle')",
        )
        .bind("/photos")
        .execute(catalog.pool())
        .await
        .unwrap();
        sqlx::query("INSERT INTO album (name, sort_mode) VALUES ('Trip', 'date:desc')")
            .execute(catalog.pool())
            .await
            .unwrap();
        sqlx::query("INSERT INTO tag (name) VALUES ('family')")
            .execute(catalog.pool())
            .await
            .unwrap();

        let counts = read_workspace_summary_counts(&workspace_root).await;
        assert_eq!(counts.root_count, 1);
        assert_eq!(counts.album_count, 1);
        assert_eq!(counts.tag_count, 1);
    }

    #[test]
    fn workspace_display_name_falls_back_for_empty_name() {
        assert_eq!(workspace_display_name(Path::new("/")), "workspace");
    }

    #[test]
    fn workspace_path_valid_checks_manifest() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("Valid");
        std::fs::create_dir_all(&root).unwrap();
        assert!(!workspace_path_valid(root.to_str().unwrap()));
        init_workspace_at(&root, false).unwrap();
        assert!(workspace_path_valid(root.to_str().unwrap()));
    }

    #[test]
    fn init_workspace_rejects_missing_path() {
        let dir = tempdir().unwrap();
        let missing = dir.path().join("missing");
        let err = init_workspace_at(&missing, false).unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn read_manifest_rejects_unsupported_schema() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("Old");
        std::fs::create_dir_all(&root).unwrap();
        let manifest = WorkspaceManifest {
            id: "ws-old".into(),
            name: "Old".into(),
            schema_version: 99,
            created_at: Utc::now().to_rfc3339(),
            read_only: false,
        };
        let paths = WorkspacePaths::new(root.clone());
        write_manifest(&paths, &manifest).unwrap();
        let err = read_manifest(&root).unwrap_err();
        assert!(err.to_string().contains("unsupported workspace schema"));
    }

    #[test]
    fn workspace_media_settings_from_paths() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("Archive");
        std::fs::create_dir_all(&root).unwrap();
        init_workspace_at(&root, true).unwrap();
        let paths = WorkspacePaths::new(root);
        let read_only = WorkspaceMediaSettings::from_paths(&paths, true);
        assert!(read_only.read_only);
        assert_eq!(read_only.workspace_xmp_dir, paths.xmp_dir());
        let editable = WorkspaceMediaSettings::from_paths(&paths, false);
        assert!(!editable.read_only);
        assert!(editable.workspace_xmp_dir.as_os_str().is_empty());
        assert!(!WorkspaceMediaSettings::default().read_only);
    }

    #[test]
    fn workspace_service_create_and_remove_recent() {
        let dir = tempdir().unwrap();
        let app_data = dir.path().join("app");
        let root = dir.path().join("Created");
        std::fs::create_dir_all(&root).unwrap();
        let mut service = WorkspaceService::load(app_data).unwrap();
        let info = service.create(&root, false).unwrap();
        assert_eq!(service.list_recent().len(), 1);
        service.remove_recent(&info.path).unwrap();
        assert!(service.list_recent().is_empty());
    }

    #[tokio::test]
    async fn enrich_recent_entry_adds_counts_and_read_only_flag() {
        use crate::catalog::Catalog;

        let dir = tempdir().unwrap();
        let root = dir.path().join("Enriched");
        std::fs::create_dir_all(&root).unwrap();
        init_workspace_at(&root, true).unwrap();
        let catalog = Catalog::open(&root.join(CATALOG_DB)).await.unwrap();
        sqlx::query(
            "INSERT INTO source_root (path, kind, scan_policy, status) VALUES (?, 'local', 'manual', 'idle')",
        )
        .bind("/photos")
        .execute(catalog.pool())
        .await
        .unwrap();

        let entry = RecentWorkspace {
            path: root.to_string_lossy().to_string(),
            name: "Enriched".into(),
            last_opened: Utc::now().timestamp(),
            valid: true,
            root_count: 0,
            album_count: 0,
            tag_count: 0,
            read_only: false,
        };
        let enriched = WorkspaceService::enrich_recent_entry(entry).await;
        assert_eq!(enriched.root_count, 1);
        assert!(enriched.read_only);
    }

    #[tokio::test]
    async fn read_workspace_summary_counts_returns_empty_for_invalid_paths() {
        use crate::catalog::Catalog;

        let dir = tempdir().unwrap();
        let plain = dir.path().join("plain");
        std::fs::create_dir_all(&plain).unwrap();
        let counts = read_workspace_summary_counts(&plain).await;
        assert_eq!(counts.root_count, 0);

        let root = dir.path().join("NoCatalog");
        std::fs::create_dir_all(&root).unwrap();
        init_workspace_at(&root, false).unwrap();
        let catalog = Catalog::open(&root.join(CATALOG_DB)).await.unwrap();
        catalog.pool().close().await;
        std::fs::remove_file(root.join(CATALOG_DB)).unwrap();
        let counts = read_workspace_summary_counts(&root).await;
        assert_eq!(counts.album_count, 0);
    }

    #[test]
    fn workspace_paths_resolve_standard_locations() {
        let root = PathBuf::from("/tmp/demo");
        let paths = WorkspacePaths::new(root.clone());
        assert_eq!(paths.manifest_path(), root.join(WORKSPACE_MANIFEST));
        assert_eq!(paths.catalog_path(), root.join(CATALOG_DB));
        assert_eq!(paths.thumbs_dir(), root.join(THUMBS_DIR));
        assert_eq!(paths.xmp_dir(), root.join(XMP_DIR));
    }

    #[test]
    fn init_workspace_rejects_non_directory_path() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("file.txt");
        std::fs::write(&file, b"x").unwrap();
        let err = init_workspace_at(&file, false).unwrap_err();
        assert!(err.to_string().contains("not a directory"));
    }

    #[test]
    fn validate_open_rejects_missing_and_non_directory_paths() {
        let dir = tempdir().unwrap();
        let app_data = dir.path().join("app");
        let service = WorkspaceService::load(app_data).unwrap();
        let err = service
            .validate_open(&dir.path().join("missing"))
            .unwrap_err();
        assert!(err.to_string().contains("not found"));
        let file = dir.path().join("file.txt");
        std::fs::write(&file, b"x").unwrap();
        let err = service.validate_open(&file).unwrap_err();
        assert!(err.to_string().contains("not a directory"));
    }

    #[test]
    fn workspace_service_exposes_registry_accessor() {
        let dir = tempdir().unwrap();
        let service = WorkspaceService::load(dir.path().join("app")).unwrap();
        assert!(service.registry().list_recent().is_empty());
    }

    #[tokio::test]
    async fn enrich_recent_entry_skips_invalid_workspace() {
        let entry = RecentWorkspace {
            path: "/missing/workspace".into(),
            name: "Missing".into(),
            last_opened: Utc::now().timestamp(),
            valid: false,
            root_count: 0,
            album_count: 0,
            tag_count: 0,
            read_only: false,
        };
        let enriched = WorkspaceService::enrich_recent_entry(entry).await;
        assert_eq!(enriched.root_count, 0);
    }

    #[tokio::test]
    async fn read_workspace_summary_counts_handles_unopenable_catalog() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("Broken");
        std::fs::create_dir_all(&root).unwrap();
        init_workspace_at(&root, false).unwrap();
        std::fs::write(root.join(CATALOG_DB), b"not-a-database").unwrap();
        let counts = read_workspace_summary_counts(&root).await;
        assert_eq!(counts.root_count, 0);
    }

    #[tokio::test]
    async fn read_workspace_summary_counts_handles_unreadable_catalog() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("UnreadableCatalog");
        std::fs::create_dir_all(&root).unwrap();
        init_workspace_at(&root, false).unwrap();
        let catalog = root.join(CATALOG_DB);
        std::fs::write(&catalog, b"not-a-database").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&catalog, std::fs::Permissions::from_mode(0o000)).unwrap();
        }
        let counts = read_workspace_summary_counts(&root).await;
        assert_eq!(counts.root_count, 0);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&catalog, std::fs::Permissions::from_mode(0o644)).unwrap();
        }
    }

    #[test]
    fn directory_is_empty_treats_missing_path_as_empty() {
        let dir = tempdir().unwrap();
        assert!(directory_is_empty(&dir.path().join("missing")).unwrap());
    }

    #[test]
    fn create_workspace_in_nested_empty_directory() {
        let dir = tempdir().unwrap();
        let app_data = dir.path().join("app");
        let mut service = WorkspaceService::load(app_data).unwrap();
        let target = dir.path().join("nested/new-workspace");
        std::fs::create_dir_all(&target).unwrap();
        let info = service.create(&target, false).unwrap();
        assert!(target.join(WORKSPACE_MANIFEST).is_file());
        assert_eq!(info.name, "new-workspace");
    }

    #[test]
    fn read_manifest_rejects_invalid_json() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("BrokenJson");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join(WORKSPACE_MANIFEST), b"{not-json").unwrap();
        let err = read_manifest(&root).unwrap_err();
        assert!(err.to_string().contains("workspace.json"));
    }

    #[cfg(unix)]
    #[test]
    fn workspace_service_touch_opened_fails_when_registry_unwritable() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempdir().unwrap();
        let app_data = dir.path().join("app");
        let root = dir.path().join("Demo");
        std::fs::create_dir_all(&root).unwrap();
        let info = init_workspace_at(&root, false).unwrap();
        let mut service = WorkspaceService::load(app_data.clone()).unwrap();
        service.touch_opened(&info).unwrap();
        let registry = app_data.join("workspaces.json");
        std::fs::set_permissions(&registry, std::fs::Permissions::from_mode(0o444)).unwrap();
        assert!(service.touch_opened(&info).is_err());
        std::fs::set_permissions(&registry, std::fs::Permissions::from_mode(0o644)).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn workspace_service_remove_recent_fails_when_registry_unwritable() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempdir().unwrap();
        let app_data = dir.path().join("app");
        let root = dir.path().join("Demo");
        std::fs::create_dir_all(&root).unwrap();
        let info = init_workspace_at(&root, false).unwrap();
        let mut service = WorkspaceService::load(app_data.clone()).unwrap();
        service.touch_opened(&info).unwrap();
        let registry = app_data.join("workspaces.json");
        std::fs::set_permissions(&registry, std::fs::Permissions::from_mode(0o444)).unwrap();
        assert!(service.remove_recent(&info.path).is_err());
        std::fs::set_permissions(&registry, std::fs::Permissions::from_mode(0o644)).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn init_workspace_propagates_read_dir_failure() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempdir().unwrap();
        let locked = dir.path().join("locked");
        std::fs::create_dir_all(&locked).unwrap();
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();
        let err = init_workspace_at(&locked, false).unwrap_err();
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert!(!err.to_string().is_empty());
    }

    #[test]
    fn workspace_info_errors_for_missing_path() {
        let err = workspace_info(&Path::new("/missing/memhg-workspace")).unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn init_workspace_at_canonicalizes_relative_path() {
        let dir = tempdir().unwrap();
        let nested = dir.path().join("nested/ws");
        std::fs::create_dir_all(&nested).unwrap();
        let info = init_workspace_at(&nested, false).unwrap();
        assert!(Path::new(&info.path).is_absolute());
    }

    #[test]
    fn write_manifest_roundtrip() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("WriteManifest");
        std::fs::create_dir_all(&root).unwrap();
        let paths = WorkspacePaths::new(root.clone());
        let manifest = WorkspaceManifest {
            id: "ws-test".into(),
            name: "WriteManifest".into(),
            schema_version: SCHEMA_VERSION,
            created_at: Utc::now().to_rfc3339(),
            read_only: false,
        };
        write_manifest(&paths, &manifest).unwrap();
        let read = read_manifest(&root).unwrap();
        assert_eq!(read.id, manifest.id);
    }

    #[test]
    fn ensure_workspace_dirs_creates_root_and_thumbs() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("EnsureDirs");
        let paths = WorkspacePaths::new(root.clone());
        ensure_workspace_dirs(&paths, false).unwrap();
        assert!(root.is_dir());
        assert!(paths.thumbs_dir().is_dir());
        assert!(!paths.xmp_dir().exists());
        ensure_workspace_dirs(&paths, true).unwrap();
        assert!(paths.xmp_dir().is_dir());
    }

    #[test]
    fn workspace_service_create_registers_recent_entry() {
        let dir = tempdir().unwrap();
        let app_data = dir.path().join("app");
        let target = dir.path().join("CreatedViaService");
        std::fs::create_dir_all(&target).unwrap();
        let mut service = WorkspaceService::load(app_data).unwrap();
        let info = service.create(&target, false).unwrap();
        assert_eq!(service.last_opened().as_deref(), Some(info.path.as_str()));
        assert_eq!(service.list_recent().len(), 1);
    }

    #[test]
    fn read_manifest_propagates_invalid_json_error() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("BadJson");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join(WORKSPACE_MANIFEST), b"{").unwrap();
        let err = read_manifest(&root).unwrap_err();
        assert!(err.to_string().contains(WORKSPACE_MANIFEST));
    }
}
