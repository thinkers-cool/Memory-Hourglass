pub mod extensions;
mod control;
pub mod index_asset;
pub mod index_integrity;
pub mod index_pipeline;

use crate::catalog::Catalog;
use crate::catalog::repo::UpsertAssetInput;
use crate::error::{AppError, Result};
use crate::metadata::metadata_context_for_asset;
use crate::workspace::WorkspaceMediaSettings;
use crate::scan::extensions::{asset_kind, is_media_file, should_ignore};
use crate::scan::index_asset::index_asset_on_disk;
use crate::scan::index_integrity::is_index_complete;
use crate::scan::index_pipeline::{apply_index_output, IndexApplyInput};
use extensions::MEDIA_EXTENSIONS;
use rayon::prelude::*;
use sqlx::SqlitePool;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use walkdir::WalkDir;

pub use control::ScanControl;

use control::{should_stop_index_batch, should_stop_inventory_batch};

pub mod test_hooks {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Mutex;

    pub static FINALIZE_SCAN_LINKS_FAIL: AtomicBool = AtomicBool::new(false);

    static ENV_TEST_LOCK: Mutex<()> = Mutex::new(());

    pub(crate) fn reset_unlocked() {
        FINALIZE_SCAN_LINKS_FAIL.store(false, Ordering::SeqCst);
        std::env::remove_var("MEMHG_TEST_INDEX_QUEUE_FAIL");
        std::env::remove_var("MEMHG_TEST_CANCEL_AFTER_PAUSE");
        std::env::remove_var("MEMHG_TEST_FORCE_INDEX_CANCEL");
        std::env::remove_var("MEMHG_TEST_FORCE_INVALID_FILE_NAME");
        std::env::remove_var("MEMHG_TEST_NULL_MTIME");
        std::env::remove_var("MEMHG_TEST_DISCOVERY_FAIL");
        std::env::remove_var("MEMHG_TEST_DISCOVERY_PANIC");
        std::env::remove_var("MEMHG_TEST_INDEX_BATCH_PANIC");
        std::env::remove_var("MEMHG_TEST_STRIP_PREFIX_FAIL");
    }

    pub fn with_env_test_lock<F, R>(f: F) -> R
    where
        F: FnOnce() -> R,
    {
        let _guard = ENV_TEST_LOCK.lock().unwrap();
        f()
    }

    pub fn env_test_guard() -> std::sync::MutexGuard<'static, ()> {
        ENV_TEST_LOCK.lock().unwrap()
    }

    pub fn reset() {
        let _guard = ENV_TEST_LOCK.lock().unwrap();
        reset_unlocked();
    }
}

const SCAN_BATCH_SIZE: usize = 250;
const INDEX_BATCH_SIZE: usize = 32;
const DISCOVERY_PROGRESS_INTERVAL: u64 = 500;

#[derive(Clone)]
pub(crate) struct PostProcessItem {
    pub asset_id: i64,
    pub root_id: i64,
    pub rel_path: String,
    pub path: PathBuf,
    pub mtime_ns: i64,
    pub kind: String,
    pub prior_thumb_key: Option<String>,
}

struct DiscoveredFile {
    rel_path: String,
    file_name: String,
    ext: String,
    kind: String,
    size: i64,
    mtime_ns: i64,
    full_path: PathBuf,
}

#[derive(Debug)]
pub struct ScanSummary {
    pub scanned: u64,
    pub indexed: u64,
    pub new_count: u64,
    pub modified_count: u64,
    pub missing_count: u64,
}

pub(crate) struct ScanInventoryResult {
    pub summary: ScanSummary,
    pub index_queue: Vec<PostProcessItem>,
    pub root_path: PathBuf,
}

pub struct ScanService {
    pub(crate) pool: SqlitePool,
    pub(crate) thumb_dir: PathBuf,
    pub(crate) media_settings: WorkspaceMediaSettings,
}

impl ScanService {
    pub fn new(pool: SqlitePool, thumb_dir: PathBuf) -> Self {
        Self::with_media_settings(pool, thumb_dir, WorkspaceMediaSettings::default())
    }

    pub fn with_media_settings(
        pool: SqlitePool,
        thumb_dir: PathBuf,
        media_settings: WorkspaceMediaSettings,
    ) -> Self {
        Self {
            pool,
            thumb_dir,
            media_settings,
        }
    }

    pub async fn scan_root(&self, root_id: i64, ctrl: &ScanControl) -> Result<ScanSummary> {
        let inventory = self.scan_inventory(root_id, ctrl, |_, _| {}).await?;
        self.process_index_queue(ctrl, &inventory.index_queue, |_, _| {})
            .await?;
        self.finalize_scan_links(root_id, &inventory.root_path).await?;
        Ok(inventory.summary)
    }

    pub(crate) async fn scan_root_with_progress<F>(
        &self,
        root_id: i64,
        ctrl: &ScanControl,
        on_progress: F,
    ) -> Result<ScanInventoryResult>
    where
        F: FnMut(u64, u64),
    {
        self.scan_inventory(root_id, ctrl, on_progress).await
    }

    pub(crate) async fn scan_inventory<F>(
        &self,
        root_id: i64,
        ctrl: &ScanControl,
        mut on_progress: F,
    ) -> Result<ScanInventoryResult>
    where
        F: FnMut(u64, u64),
    {
        let catalog = Catalog::from_pool(self.pool.clone());
        let roots = catalog.roots();
        let root = roots.get_root(root_id).await?;
        let root_path = PathBuf::from(&root.path);
        if !root_path.is_dir() {
            roots.set_status(root_id, "offline").await?;
            return Ok(ScanInventoryResult {
                summary: ScanSummary {
                    scanned: 0,
                    indexed: 0,
                    new_count: 0,
                    modified_count: 0,
                    missing_count: 0,
                },
                index_queue: Vec::new(),
                root_path,
            });
        }

        roots.set_status(root_id, "scanning").await?;

        let assets = catalog.assets();
        let existing = assets.list_scan_state_for_root(root_id).await?;
        let existing_by_path: HashMap<String, _> = existing
            .into_iter()
            .map(|row| (row.rel_path.clone(), row))
            .collect();

        let deleted_paths: HashSet<String> = assets
            .list_deleted_paths_for_root(root_id)
            .await?
            .into_iter()
            .collect();

        let discovered_count = Arc::new(AtomicU64::new(0));
        let cancelled = ctrl.cancelled_flag();
        let root_path_for_walk = root_path.clone();
        let collect_handle = tokio::task::spawn_blocking({
            let discovered_count = discovered_count.clone();
            move || collect_discovered_files(&root_path_for_walk, cancelled, discovered_count)
        });

        while !collect_handle.is_finished() {
            tokio::time::sleep(Duration::from_millis(DISCOVERY_PROGRESS_INTERVAL)).await;
            let discovered = discovered_count.load(Ordering::Relaxed);
            if discovered > 0 {
                on_progress(discovered, 0);
            }
            if ctrl.is_cancelled() {
                collect_handle.abort();
                break;
            }
        }

        let discovered = collect_handle
            .await
            .map_err(|error| AppError::Scan(format!("discovery failed: {}", error)))??;

        let mut seen = HashMap::new();
        let mut summary = ScanSummary {
            scanned: 0,
            indexed: 0,
            new_count: 0,
            modified_count: 0,
            missing_count: 0,
        };
        let mut index_queue = Vec::new();

        for batch in discovered.chunks(SCAN_BATCH_SIZE) {
            if should_stop_inventory_batch(ctrl) { break; }
            ctrl.wait_if_paused().await;
            if should_stop_inventory_batch(ctrl) { break; }

            let mut batch_inputs = Vec::new();
            let mut batch_meta: Vec<(&DiscoveredFile, bool, Option<String>)> = Vec::new();

            for entry in batch {
                seen.insert(entry.rel_path.clone(), (entry.mtime_ns, entry.size));
                summary.scanned += 1;

                if deleted_paths.contains(&entry.rel_path) {
                    continue;
                }

                let sync_state = match existing_by_path.get(&entry.rel_path) {
                    None => {
                        summary.new_count += 1;
                        "new"
                    }
                    Some(row) if row.mtime_ns != entry.mtime_ns || row.size != entry.size => {
                        summary.modified_count += 1;
                        "modified"
                    }
                    _ => "ok",
                };

                let prior = existing_by_path.get(&entry.rel_path);
                let needs_index = !is_index_complete(
                    entry.mtime_ns,
                    prior.and_then(|row| row.indexed_mtime_ns),
                    prior.and_then(|row| row.thumb_key.as_deref()),
                    &entry.kind,
                );

                batch_inputs.push(UpsertAssetInput {
                    root_id,
                    rel_path: &entry.rel_path,
                    file_name: &entry.file_name,
                    ext: &entry.ext,
                    kind: &entry.kind,
                    size: entry.size,
                    mtime_ns: entry.mtime_ns,
                    sync_state,
                });
                batch_meta.push((
                    entry,
                    needs_index,
                    prior.and_then(|row| row.thumb_key.clone()),
                ));
            }

            let asset_ids = assets.upsert_assets_batch(root_id, &batch_inputs).await?;
            for ((entry, needs_index, prior_thumb_key), asset_id) in
                batch_meta.into_iter().zip(asset_ids)
            {
                summary.indexed += 1;
                if needs_index {
                    index_queue.push(PostProcessItem {
                        asset_id,
                        root_id,
                        rel_path: entry.rel_path.clone(),
                        path: entry.full_path.clone(),
                        mtime_ns: entry.mtime_ns,
                        kind: entry.kind.clone(),
                        prior_thumb_key,
                    });
                }
            }

            on_progress(summary.scanned, summary.indexed);
            tokio::task::yield_now().await;
        }

        let missing: Vec<String> = existing_by_path
            .keys()
            .filter(|path| !seen.contains_key(path.as_str()))
            .cloned()
            .collect();
        summary.missing_count = missing.len() as u64;
        assets.mark_missing(root_id, &missing).await?;

        let now = chrono::Utc::now().timestamp();
        roots.touch_scan(root_id, now).await?;
        roots.set_status(root_id, "idle").await?;

        on_progress(summary.scanned, summary.indexed);

        Ok(ScanInventoryResult {
            summary,
            index_queue,
            root_path,
        })
    }

    pub(crate) async fn process_index_queue<F>(
        &self,
        ctrl: &ScanControl,
        index_queue: &[PostProcessItem],
        mut on_progress: F,
    ) -> Result<()>
    where
        F: FnMut(u64, u64),
    {
        let total = index_queue.len() as u64;
        if total == 0 {
            return Ok(());
        }

        if control::take_env_cancel_flag("MEMHG_TEST_INDEX_QUEUE_FAIL") {
            return Err(AppError::Scan("index queue failed".into()));
        }

        let mut processed = 0u64;
        for batch in index_queue.chunks(INDEX_BATCH_SIZE) {
            if should_stop_index_batch(ctrl) {
                return Ok(());
            }
            ctrl.wait_if_paused().await;
            if should_stop_index_batch(ctrl) {
                return Ok(());
            }

            let thumb_dir = self.thumb_dir.clone();
            let media_settings = self.media_settings.clone();
            let batch_items = batch.to_vec();
            let indexed_batch = tokio::task::spawn_blocking(move || {
                if std::env::var_os("MEMHG_TEST_INDEX_BATCH_PANIC").is_some() {
                    std::env::remove_var("MEMHG_TEST_INDEX_BATCH_PANIC");
                    panic!("index batch panic");
                }
                batch_items
                    .par_iter()
                    .map(|item| {
                        let metadata_ctx = metadata_context_for_asset(
                            media_settings.read_only,
                            &media_settings.workspace_xmp_dir,
                            item.root_id,
                            &item.rel_path,
                            item.path.clone(),
                        );
                        let indexed = index_asset_on_disk(
                            item.asset_id,
                            &item.path,
                            &thumb_dir,
                            &metadata_ctx,
                        );
                        (
                            item.asset_id,
                            item.mtime_ns,
                            item.kind.clone(),
                            item.prior_thumb_key.clone(),
                            indexed,
                        )
                    })
                    .collect::<Vec<_>>()
            })
            .await
            .map_err(|error| AppError::Scan(format!("index batch failed: {}", error)))?;

            for (asset_id, mtime_ns, kind, prior_thumb_key, indexed) in indexed_batch {
                apply_index_output(
                    &self.pool,
                    &IndexApplyInput {
                        asset_id,
                        mtime_ns,
                        kind,
                        prior_thumb_key,
                        indexed,
                    },
                )
                .await?;
            }

            processed += batch.len() as u64;
            on_progress(processed, total);
            tokio::task::yield_now().await;
        }

        Ok(())
    }

    pub async fn finalize_scan_links(&self, root_id: i64, root_path: &Path) -> Result<()> {
        #[cfg(test)]
        if test_hooks::FINALIZE_SCAN_LINKS_FAIL.load(Ordering::SeqCst) {
            return Err(AppError::Scan("finalize scan links failed".into()));
        }
        let link = crate::link::LinkService::new(self.pool.clone());
        link.link_raw_jpeg_in_root(root_id).await?;
        link.compute_hashes_for_root(root_id, root_path).await?;
        link.rebuild_duplicate_links().await?;
        link.refresh_duplicate_flags().await?;
        Ok(())
    }
}

fn collect_discovered_files(
    root_path: &Path,
    cancelled: Arc<std::sync::atomic::AtomicBool>,
    discovered_count: Arc<AtomicU64>,
) -> Result<Vec<DiscoveredFile>> {
    if std::env::var_os("MEMHG_TEST_DISCOVERY_FAIL").is_some() {
        std::env::remove_var("MEMHG_TEST_DISCOVERY_FAIL");
        return Err(AppError::Scan("discovery failed".into()));
    }
    if std::env::var_os("MEMHG_TEST_DISCOVERY_PANIC").is_some() {
        std::env::remove_var("MEMHG_TEST_DISCOVERY_PANIC");
        panic!("discovery panic");
    }
    let mut entries = Vec::new();
    for entry in WalkDir::new(root_path)
        .follow_links(false)
        .into_iter()
        .filter_map(|entry| entry.ok())
    {
        if cancelled.load(Ordering::Relaxed) {
            break;
        }

        let path = entry.path();
        if path.is_dir() || should_ignore(path) || !is_media_file(path) {
            continue;
        }

        let meta = std::fs::metadata(path)?;
        let file_name = match valid_discovered_file_name(path) {
            Some(name) => name,
            None => continue,
        };
        let mtime_ns = match file_mtime_ns(&meta) {
            Some(ns) => ns,
            None => continue,
        };
        let size = meta.len() as i64;
        if std::env::var_os("MEMHG_TEST_STRIP_PREFIX_FAIL").is_some()
            && path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name == "strip-fail.jpg")
        {
            std::env::remove_var("MEMHG_TEST_STRIP_PREFIX_FAIL");
            return Err(AppError::Scan("strip prefix failed".into()));
        }
        let strip_root = if std::env::var_os("MEMHG_TEST_STRIP_PREFIX_MAP_ERR").is_some()
            && path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name == "strip-map.jpg")
        {
            std::env::remove_var("MEMHG_TEST_STRIP_PREFIX_MAP_ERR");
            Path::new("/memhg-strip-prefix-mismatch")
        } else {
            root_path
        };
        let rel = path
            .strip_prefix(strip_root)
            .map_err(|error| AppError::Scan(error.to_string()))?
            .to_string_lossy()
            .replace('\\', "/");
        let ext = path
            .extension()
            .and_then(|value| value.to_str())
            .map(|value| value.to_lowercase())
            .unwrap_or_default();
        let kind = asset_kind(&ext).to_string();

        entries.push(DiscoveredFile {
            rel_path: rel,
            file_name,
            ext,
            kind,
            size,
            mtime_ns,
            full_path: path.to_path_buf(),
        });

        let count = entries.len() as u64;
        if count.is_multiple_of(250) {
            discovered_count.store(count, Ordering::Relaxed);
        }
    }

    discovered_count.store(entries.len() as u64, Ordering::Relaxed);
    Ok(entries)
}

fn valid_discovered_file_name(path: &Path) -> Option<String> {
    if std::env::var_os("MEMHG_TEST_FORCE_INVALID_FILE_NAME").is_some()
        && path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name == "force-invalid.jpg")
    {
        std::env::remove_var("MEMHG_TEST_FORCE_INVALID_FILE_NAME");
        return None;
    }
    match path.file_name().and_then(|name| name.to_str()) {
        Some(name) if !name.is_empty() => Some(name.to_string()),
        _ => None,
    }
}

pub fn file_mtime_ns(meta: &std::fs::Metadata) -> Option<i64> {
    if std::env::var_os("MEMHG_TEST_NULL_MTIME").is_some() {
        return None;
    }
    meta.modified()
        .ok()
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|duration| duration.as_nanos() as i64)
}

pub fn list_supported_extensions() -> &'static [&'static str] {
    MEDIA_EXTENSIONS
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::repo::{AssetRepo, RawTagRepo, SourceRootRepo};
    use crate::catalog::Catalog;
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;
    use tempfile::tempdir;

    struct ScanTestGuard {
        _lock: std::sync::MutexGuard<'static, ()>,
    }

    impl ScanTestGuard {
        fn new() -> Self {
            let lock = test_hooks::env_test_guard();
            test_hooks::reset_unlocked();
            Self { _lock: lock }
        }
    }

    impl Drop for ScanTestGuard {
        fn drop(&mut self) {
            test_hooks::reset_unlocked();
        }
    }

    fn noop_scan_progress(_discovered: u64, _indexed: u64) {
        std::hint::black_box(());
    }

    #[tokio::test]
    async fn scan_indexes_jpeg_in_directory() {
        let _guard = ScanTestGuard::new();
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        let jpeg = include_bytes!("../../tests/fixtures/minimal.jpg");
        std::fs::write(photos.join("test.jpg"), jpeg).unwrap();

        let db = dir.path().join("catalog.db");
        let thumb_dir = dir.path().join("thumbs");
        let catalog = Catalog::open(&db).await.unwrap();

        let roots = SourceRootRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();

        let scanner = ScanService::new(catalog.pool().clone(), thumb_dir);
        let summary = scanner.scan_root(root.id, &ScanControl::noop()).await.unwrap();

        assert_eq!(summary.indexed, 1);
        assert_eq!(summary.new_count, 1);

        let assets = AssetRepo::new(catalog.pool().clone());
        let asset = assets.find_by_path(root.id, "test.jpg").await.unwrap().unwrap();
        assert_eq!(asset.sync_state, "ok");
        assert_eq!(asset.indexed_mtime_ns, Some(asset.mtime_ns));
    }

    #[tokio::test]
    async fn scan_reindexes_ok_asset_without_index_marker() {
        let _guard = ScanTestGuard::new();
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        let jpeg = include_bytes!("../../tests/fixtures/minimal.jpg");
        std::fs::write(photos.join("test.jpg"), jpeg).unwrap();

        let db = dir.path().join("catalog.db");
        let thumb_dir = dir.path().join("thumbs");
        let catalog = Catalog::open(&db).await.unwrap();

        let roots = SourceRootRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();

        let assets = AssetRepo::new(catalog.pool().clone());
        let raw_tag_repo = RawTagRepo::new(catalog.pool().clone());
        let asset = assets
            .upsert_asset(UpsertAssetInput {
                root_id: root.id,
                rel_path: "test.jpg",
                file_name: "test.jpg",
                ext: "jpg",
                kind: "image",
                size: 1,
                mtime_ns: 1,
                sync_state: "ok",
            })
            .await
            .unwrap();
        assets.set_sync_state(asset.id, "ok").await.unwrap();

        assert_eq!(raw_tag_repo.list_for_asset(asset.id).await.unwrap().len(), 0);

        let scanner = ScanService::new(catalog.pool().clone(), thumb_dir);
        scanner.scan_root(root.id, &ScanControl::noop()).await.unwrap();

        let asset = assets.find_by_path(root.id, "test.jpg").await.unwrap().unwrap();
        assert_eq!(asset.sync_state, "ok");
        assert_eq!(asset.indexed_mtime_ns, Some(asset.mtime_ns));
        assert!(asset.thumb_key.is_some());
        assert!(!raw_tag_repo.list_for_asset(asset.id).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn scan_skips_soft_deleted_assets() {
        let _guard = ScanTestGuard::new();
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        let jpeg = include_bytes!("../../tests/fixtures/minimal.jpg");
        std::fs::write(photos.join("trashed.jpg"), jpeg).unwrap();

        let db = dir.path().join("catalog.db");
        let thumb_dir = dir.path().join("thumbs");
        let catalog = Catalog::open(&db).await.unwrap();

        let roots = SourceRootRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();

        let scanner = ScanService::new(catalog.pool().clone(), thumb_dir);
        scanner
            .scan_root(root.id, &ScanControl::noop())
            .await
            .unwrap();

        let assets = AssetRepo::new(catalog.pool().clone());
        let asset = assets
            .find_by_path(root.id, "trashed.jpg")
            .await
            .unwrap()
            .unwrap();
        assets.soft_delete(&[asset.id], 1).await.unwrap();

        scanner
            .scan_root(root.id, &ScanControl::noop())
            .await
            .unwrap();

        assert!(assets.find_by_path(root.id, "trashed.jpg").await.unwrap().is_none());
        let stored = assets
            .find_by_path_including_deleted(root.id, "trashed.jpg")
            .await
            .unwrap()
            .unwrap();
        assert!(stored.deleted_at.is_some());
    }

    #[test]
    fn valid_discovered_file_name_rejects_empty_names() {
        let _guard = ScanTestGuard::new();
        assert!(valid_discovered_file_name(Path::new("/tmp/photo.jpg")).is_some());
        assert!(valid_discovered_file_name(Path::new("/")).is_none());
    }

    #[test]
    fn valid_discovered_file_name_honors_force_invalid_env() {
        let _guard = ScanTestGuard::new();
        std::env::set_var("MEMHG_TEST_FORCE_INVALID_FILE_NAME", "1");
        assert!(valid_discovered_file_name(Path::new("/tmp/force-invalid.jpg")).is_none());
        assert!(!std::env::var_os("MEMHG_TEST_FORCE_INVALID_FILE_NAME").is_some());
        assert!(valid_discovered_file_name(Path::new("/tmp/ok.jpg")).is_some());
    }

    #[test]
    fn file_mtime_ns_returns_none_when_test_env_set() {
        let _guard = ScanTestGuard::new();
        std::env::set_var("MEMHG_TEST_NULL_MTIME", "1");
        let dir = tempdir().unwrap();
        let path = dir.path().join("mtime.jpg");
        std::fs::write(&path, b"x").unwrap();
        let meta = std::fs::metadata(&path).unwrap();
        assert!(file_mtime_ns(&meta).is_none());
    }

    #[tokio::test]
    async fn process_index_queue_fails_when_test_env_set() {
        let _guard = ScanTestGuard::new();
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let scanner = ScanService::new(catalog.pool().clone(), dir.path().join("thumbs"));
        let item = PostProcessItem {
            asset_id: 1,
            root_id: 1,
            rel_path: "a.jpg".into(),
            path: dir.path().join("a.jpg"),
            mtime_ns: 1,
            kind: "image".into(),
            prior_thumb_key: None,
        };
        std::env::set_var("MEMHG_TEST_INDEX_QUEUE_FAIL", "1");
        let err = scanner
            .process_index_queue(&ScanControl::noop(), &[item], noop_scan_progress)
            .await
            .unwrap_err();
        assert!(err.to_string().contains("index queue failed"));
    }

    #[tokio::test]
    async fn process_index_queue_returns_early_when_index_cancel_env_set() {
        let _guard = ScanTestGuard::new();
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let scanner = ScanService::new(catalog.pool().clone(), dir.path().join("thumbs"));
        let item = PostProcessItem {
            asset_id: 1,
            root_id: 1,
            rel_path: "a.jpg".into(),
            path: dir.path().join("a.jpg"),
            mtime_ns: 1,
            kind: "image".into(),
            prior_thumb_key: None,
        };
        std::env::set_var("MEMHG_TEST_FORCE_INDEX_CANCEL", "1");
        scanner
            .process_index_queue(&ScanControl::noop(), &[item], noop_scan_progress)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn finalize_scan_links_fails_when_hook_enabled() {
        let _guard = ScanTestGuard::new();
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let scanner = ScanService::new(catalog.pool().clone(), dir.path().join("thumbs"));
        test_hooks::FINALIZE_SCAN_LINKS_FAIL.store(true, Ordering::SeqCst);
        let err = scanner
            .finalize_scan_links(1, dir.path())
            .await
            .unwrap_err();
        assert!(err.to_string().contains("finalize scan links failed"));
    }

    #[tokio::test]
    async fn scan_inventory_stops_when_cancel_env_set() {
        let _guard = ScanTestGuard::new();
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        for index in 0..3 {
            std::fs::write(
                photos.join(format!("img-{index}.jpg")),
                include_bytes!("../../tests/fixtures/minimal.jpg"),
            )
            .unwrap();
        }
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        let scanner = ScanService::new(catalog.pool().clone(), dir.path().join("thumbs"));
        std::env::set_var("MEMHG_TEST_CANCEL_AFTER_PAUSE", "1");
        let result = scanner
            .scan_inventory(root.id, &ScanControl::noop(), |_, _| {})
            .await
            .unwrap();
        assert!(result.summary.scanned <= 3);
    }

    #[tokio::test]
    async fn scan_inventory_counts_modified_assets() {
        let _guard = ScanTestGuard::new();
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        let file = photos.join("track.jpg");
        std::fs::write(&file, include_bytes!("../../tests/fixtures/minimal.jpg")).unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        let scanner = ScanService::new(catalog.pool().clone(), dir.path().join("thumbs"));
        scanner
            .scan_inventory(root.id, &ScanControl::noop(), |_, _| {})
            .await
            .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
        std::fs::write(&file, b"changed-bytes").unwrap();
        let result = scanner
            .scan_inventory(root.id, &ScanControl::noop(), |_, _| {})
            .await
            .unwrap();
        assert!(result.summary.modified_count >= 1);
    }

    #[tokio::test]
    async fn scan_root_with_progress_reports_inventory() {
        let _guard = ScanTestGuard::new();
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        std::fs::write(
            photos.join("one.jpg"),
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        let scanner = ScanService::new(catalog.pool().clone(), dir.path().join("thumbs"));
        let mut progress = 0u64;
        let result = scanner
            .scan_root_with_progress(root.id, &ScanControl::noop(), |discovered, _| {
                progress = discovered;
            })
            .await
            .unwrap();
        assert!(result.summary.scanned >= 1);
        assert!(progress >= 1);
    }

    #[tokio::test]
    async fn scan_inventory_fails_when_discovery_errors() {
        let _guard = ScanTestGuard::new();
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        std::fs::write(
            photos.join("one.jpg"),
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        std::env::set_var("MEMHG_TEST_DISCOVERY_FAIL", "1");
        let scanner = ScanService::new(catalog.pool().clone(), dir.path().join("thumbs"));
        let result = scanner
            .scan_inventory(root.id, &ScanControl::noop(), noop_scan_progress)
            .await;
        assert!(result.is_err(), "expected discovery failure");
        assert!(
            result
                .err()
                .expect("discovery error")
                .to_string()
                .contains("discovery failed")
        );
    }

    #[tokio::test]
    async fn scan_inventory_marks_offline_root() {
        let _guard = ScanTestGuard::new();
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let missing = dir.path().join("missing-root");
        let root = roots
            .insert_root(missing.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        let scanner = ScanService::new(catalog.pool().clone(), dir.path().join("thumbs"));
        let result = scanner
            .scan_inventory(root.id, &ScanControl::noop(), noop_scan_progress)
            .await
            .unwrap();
        assert_eq!(result.summary.scanned, 0);
        let updated = roots.get_root(root.id).await.unwrap();
        assert_eq!(updated.status, "offline");
    }

    #[test]
    fn list_supported_extensions_exposes_media_extensions() {
        let _guard = ScanTestGuard::new();
        let extensions = list_supported_extensions();
        assert!(extensions.contains(&"jpg"));
        assert!(extensions.contains(&"cr2"));
    }

    #[test]
    fn collect_discovered_files_stops_when_cancelled() {
        let _guard = ScanTestGuard::new();
        let dir = tempdir().unwrap();
        std::fs::write(
            dir.path().join("a.jpg"),
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();
        let cancelled = Arc::new(AtomicBool::new(true));
        let discovered_count = Arc::new(AtomicU64::new(0));
        let entries =
            collect_discovered_files(dir.path(), cancelled, discovered_count).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn collect_discovered_files_skips_null_mtime_files() {
        let _guard = ScanTestGuard::new();
        std::env::set_var("MEMHG_TEST_NULL_MTIME", "1");
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("skip.jpg"), b"x").unwrap();
        let cancelled = Arc::new(AtomicBool::new(false));
        let discovered_count = Arc::new(AtomicU64::new(0));
        let entries =
            collect_discovered_files(dir.path(), cancelled, discovered_count).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn collect_discovered_files_updates_progress_every_250_files() {
        let _guard = ScanTestGuard::new();
        let dir = tempdir().unwrap();
        let jpeg = include_bytes!("../../tests/fixtures/minimal.jpg");
        for index in 0..251 {
            std::fs::write(dir.path().join(format!("img-{index}.jpg")), jpeg).unwrap();
        }
        let cancelled = Arc::new(AtomicBool::new(false));
        let discovered_count = Arc::new(AtomicU64::new(0));
        let entries = collect_discovered_files(
            dir.path(),
            cancelled,
            discovered_count.clone(),
        )
        .unwrap();
        assert_eq!(entries.len(), 251);
        assert_eq!(discovered_count.load(Ordering::Relaxed), 251);
    }

    #[tokio::test]
    async fn scan_inventory_aborts_discovery_when_cancelled() {
        let _guard = ScanTestGuard::new();
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        let jpeg = include_bytes!("../../tests/fixtures/minimal.jpg");
        for index in 0..40 {
            std::fs::write(photos.join(format!("img-{index}.jpg")), jpeg).unwrap();
        }
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        let cancelled = Arc::new(AtomicBool::new(false));
        let ctrl = ScanControl::new(Arc::new(AtomicBool::new(false)), cancelled.clone());
        let scanner = ScanService::new(catalog.pool().clone(), dir.path().join("thumbs"));
        let handle = tokio::spawn(async move {
            scanner
                .scan_inventory(root.id, &ctrl, |_, _| {})
                .await
        });
        tokio::time::sleep(Duration::from_millis(50)).await;
        cancelled.store(true, Ordering::SeqCst);
        handle.await.unwrap().unwrap();
    }

    #[tokio::test]
    async fn scan_inventory_stops_after_pause_when_cancelled() {
        let _guard = ScanTestGuard::new();
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        let jpeg = include_bytes!("../../tests/fixtures/minimal.jpg");
        for index in 0..3 {
            std::fs::write(photos.join(format!("img-{index}.jpg")), jpeg).unwrap();
        }
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        let paused = Arc::new(AtomicBool::new(true));
        let cancelled = Arc::new(AtomicBool::new(false));
        let ctrl = ScanControl::new(paused.clone(), cancelled.clone());
        let scanner = ScanService::new(catalog.pool().clone(), dir.path().join("thumbs"));
        let handle = tokio::spawn(async move {
            scanner
                .scan_inventory(root.id, &ctrl, |_, _| {})
                .await
        });
        tokio::time::sleep(Duration::from_millis(100)).await;
        cancelled.store(true, Ordering::SeqCst);
        paused.store(false, Ordering::SeqCst);
        handle.await.unwrap().unwrap();
    }

    #[tokio::test]
    async fn process_index_queue_stops_after_pause_when_cancelled() {
        let _guard = ScanTestGuard::new();
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let scanner = ScanService::new(catalog.pool().clone(), dir.path().join("thumbs"));
        let paused = Arc::new(AtomicBool::new(true));
        let cancelled = Arc::new(AtomicBool::new(false));
        let ctrl = ScanControl::new(paused.clone(), cancelled.clone());
        let item = PostProcessItem {
            asset_id: 1,
            root_id: 1,
            rel_path: "a.jpg".into(),
            path: dir.path().join("a.jpg"),
            mtime_ns: 1,
            kind: "image".into(),
            prior_thumb_key: None,
        };
        let handle = tokio::spawn(async move {
            scanner
                .process_index_queue(&ctrl, &[item], noop_scan_progress)
                .await
        });
        cancelled.store(true, Ordering::SeqCst);
        paused.store(false, Ordering::SeqCst);
        handle.await.unwrap().unwrap();
    }

    #[tokio::test]
    async fn scan_inventory_cancels_after_unpause_before_next_batch() {
        let _guard = ScanTestGuard::new();
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        for index in 0..5 {
            std::fs::write(
                photos.join(format!("img-{index}.jpg")),
                include_bytes!("../../tests/fixtures/minimal.jpg"),
            )
            .unwrap();
        }
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        let paused = Arc::new(AtomicBool::new(true));
        let cancelled = Arc::new(AtomicBool::new(false));
        let ctrl = ScanControl::new(paused.clone(), cancelled.clone());
        let scanner = ScanService::new(catalog.pool().clone(), dir.path().join("thumbs"));
        let handle = tokio::spawn(async move {
            scanner
                .scan_inventory(root.id, &ctrl, |_, _| {})
                .await
        });
        tokio::time::sleep(Duration::from_millis(50)).await;
        paused.store(false, Ordering::SeqCst);
        tokio::time::sleep(Duration::from_millis(10)).await;
        cancelled.store(true, Ordering::SeqCst);
        handle.await.unwrap().unwrap();
    }

    #[tokio::test]
    async fn process_index_queue_cancels_after_unpause_before_next_batch() {
        let _guard = ScanTestGuard::new();
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let scanner = ScanService::new(catalog.pool().clone(), dir.path().join("thumbs"));
        let paused = Arc::new(AtomicBool::new(true));
        let cancelled = Arc::new(AtomicBool::new(false));
        let ctrl = ScanControl::new(paused.clone(), cancelled.clone());
        let items = (0..4)
            .map(|index| PostProcessItem {
                asset_id: index + 1,
                root_id: 1,
                rel_path: format!("a-{index}.jpg"),
                path: dir.path().join(format!("a-{index}.jpg")),
                mtime_ns: 1,
                kind: "image".into(),
                prior_thumb_key: None,
            })
            .collect::<Vec<_>>();
        let handle = tokio::spawn(async move {
            scanner
                .process_index_queue(&ctrl, &items, noop_scan_progress)
                .await
        });
        tokio::time::sleep(Duration::from_millis(50)).await;
        paused.store(false, Ordering::SeqCst);
        tokio::time::sleep(Duration::from_millis(10)).await;
        cancelled.store(true, Ordering::SeqCst);
        handle.await.unwrap().unwrap();
    }

    #[tokio::test]
    async fn scan_inventory_checks_cancel_after_pause_twice() {
        let _guard = ScanTestGuard::new();
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        for index in 0..5 {
            std::fs::write(
                photos.join(format!("img-{index}.jpg")),
                include_bytes!("../../tests/fixtures/minimal.jpg"),
            )
            .unwrap();
        }
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        let paused = Arc::new(AtomicBool::new(true));
        let cancelled = Arc::new(AtomicBool::new(false));
        let ctrl = ScanControl::new(paused.clone(), cancelled.clone());
        let scanner = ScanService::new(catalog.pool().clone(), dir.path().join("thumbs"));
        let handle = tokio::spawn(async move {
            scanner
                .scan_inventory(root.id, &ctrl, |_, _| {})
                .await
        });
        tokio::time::sleep(Duration::from_millis(50)).await;
        cancelled.store(true, Ordering::SeqCst);
        paused.store(false, Ordering::SeqCst);
        handle.await.unwrap().unwrap();
    }

    #[test]
    fn collect_discovered_files_skips_force_invalid_filename() {
        let _guard = ScanTestGuard::new();
        std::env::set_var("MEMHG_TEST_FORCE_INVALID_FILE_NAME", "1");
        let dir = tempdir().unwrap();
        std::fs::write(
            dir.path().join("force-invalid.jpg"),
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();
        std::fs::write(
            dir.path().join("ok.jpg"),
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();
        let cancelled = Arc::new(AtomicBool::new(false));
        let discovered_count = Arc::new(AtomicU64::new(0));
        let entries =
            collect_discovered_files(dir.path(), cancelled, discovered_count).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].file_name, "ok.jpg");
    }

    #[tokio::test]
    async fn scan_inventory_marks_removed_files_missing() {
        let _guard = ScanTestGuard::new();
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        let file = photos.join("vanish.jpg");
        std::fs::write(&file, include_bytes!("../../tests/fixtures/minimal.jpg")).unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let assets = AssetRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        let scanner = ScanService::new(catalog.pool().clone(), dir.path().join("thumbs"));
        scanner
            .scan_inventory(root.id, &ScanControl::noop(), |_, _| {})
            .await
            .unwrap();
        std::fs::remove_file(&file).unwrap();
        let result = scanner
            .scan_inventory(root.id, &ScanControl::noop(), |_, _| {})
            .await
            .unwrap();
        assert_eq!(result.summary.missing_count, 1);
        let asset = assets.find_by_path(root.id, "vanish.jpg").await.unwrap().unwrap();
        assert_eq!(asset.sync_state, "missing");
    }

    #[tokio::test]
    async fn scan_inventory_skips_index_queue_when_already_indexed() {
        let _guard = ScanTestGuard::new();
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        let file = photos.join("indexed.jpg");
        std::fs::write(&file, include_bytes!("../../tests/fixtures/minimal.jpg")).unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        let scanner = ScanService::new(catalog.pool().clone(), dir.path().join("thumbs"));
        scanner
            .scan_root(root.id, &ScanControl::noop())
            .await
            .unwrap();
        let result = scanner
            .scan_inventory(root.id, &ScanControl::noop(), |_, _| {})
            .await
            .unwrap();
        assert!(result.index_queue.is_empty());
        assert_eq!(result.summary.modified_count, 0);
        assert_eq!(result.summary.new_count, 0);
    }

    #[tokio::test]
    async fn scan_inventory_detects_size_only_modification() {
        let _guard = ScanTestGuard::new();
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        let file = photos.join("size.jpg");
        std::fs::write(&file, include_bytes!("../../tests/fixtures/minimal.jpg")).unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let assets = AssetRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        let scanner = ScanService::new(catalog.pool().clone(), dir.path().join("thumbs"));
        scanner
            .scan_inventory(root.id, &ScanControl::noop(), |_, _| {})
            .await
            .unwrap();
        let asset = assets.find_by_path(root.id, "size.jpg").await.unwrap().unwrap();
        sqlx::query("UPDATE asset SET size = 1 WHERE id = ?")
            .bind(asset.id)
            .execute(catalog.pool())
            .await
            .unwrap();
        let result = scanner
            .scan_inventory(root.id, &ScanControl::noop(), |_, _| {})
            .await
            .unwrap();
        assert!(result.summary.modified_count >= 1);
    }

    #[tokio::test]
    async fn process_index_queue_noop_on_empty_queue() {
        let _guard = ScanTestGuard::new();
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let scanner = ScanService::new(catalog.pool().clone(), dir.path().join("thumbs"));
        scanner
            .process_index_queue(&ScanControl::noop(), &[], noop_scan_progress)
            .await
            .unwrap();
        noop_scan_progress(0, 0);
    }

    #[test]
    fn file_mtime_ns_returns_some_for_real_file() {
        let _guard = ScanTestGuard::new();
        let dir = tempdir().unwrap();
        let path = dir.path().join("mtime.jpg");
        std::fs::write(&path, b"x").unwrap();
        let meta = std::fs::metadata(&path).unwrap();
        assert!(file_mtime_ns(&meta).is_some());
    }

    #[test]
    fn collect_discovered_files_skips_non_media_entries() {
        let _guard = ScanTestGuard::new();
        let dir = tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("subdir")).unwrap();
        std::fs::write(dir.path().join("notes.txt"), b"txt").unwrap();
        std::fs::write(
            dir.path().join("photo.jpg"),
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();
        let cancelled = Arc::new(AtomicBool::new(false));
        let discovered_count = Arc::new(AtomicU64::new(0));
        let entries =
            collect_discovered_files(dir.path(), cancelled, discovered_count).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].file_name, "photo.jpg");
    }

    #[tokio::test]
    async fn scan_root_propagates_finalize_failure() {
        let _guard = ScanTestGuard::new();
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        std::fs::write(
            photos.join("photo.jpg"),
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        let scanner = ScanService::new(catalog.pool().clone(), dir.path().join("thumbs"));
        test_hooks::FINALIZE_SCAN_LINKS_FAIL.store(true, Ordering::SeqCst);
        let result = scanner.scan_root(root.id, &ScanControl::noop()).await;
        let err = result.err().expect("expected finalize failure");
        assert!(err.to_string().contains("finalize scan links failed"));
    }

    #[tokio::test]
    async fn scan_root_finalizes_links_and_hashes() {
        let _guard = ScanTestGuard::new();
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        let jpeg = include_bytes!("../../tests/fixtures/minimal.jpg");
        std::fs::write(photos.join("dup-a.jpg"), jpeg).unwrap();
        std::fs::write(photos.join("dup-b.jpg"), jpeg).unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let pool = catalog.pool().clone();
        let roots = SourceRootRepo::new(pool.clone());
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        let scanner = ScanService::new(pool.clone(), dir.path().join("thumbs"));
        scanner.scan_root(root.id, &ScanControl::noop()).await.unwrap();
        let hashed: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM asset WHERE root_id = ? AND content_hash IS NOT NULL",
        )
        .bind(root.id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(hashed, 2);
        let flagged: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM asset WHERE root_id = ? AND has_duplicate = 1",
        )
        .bind(root.id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(flagged, 2);
    }

    #[tokio::test]
    async fn scan_inventory_emits_discovery_progress() {
        let _guard = ScanTestGuard::new();
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        for index in 0..3 {
            std::fs::write(
                photos.join(format!("img-{index}.jpg")),
                include_bytes!("../../tests/fixtures/minimal.jpg"),
            )
            .unwrap();
        }
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        let scanner = ScanService::new(catalog.pool().clone(), dir.path().join("thumbs"));
        let mut saw_discovery = false;
        scanner
            .scan_inventory(root.id, &ScanControl::noop(), |discovered, indexed| {
                if discovered > 0 && indexed == 0 {
                    saw_discovery = true;
                }
            })
            .await
            .unwrap();
        assert!(saw_discovery);
    }

    #[test]
    fn collect_discovered_files_fails_when_discovery_env_set() {
        let _guard = ScanTestGuard::new();
        std::env::set_var("MEMHG_TEST_DISCOVERY_FAIL", "1");
        let dir = tempdir().unwrap();
        let cancelled = Arc::new(AtomicBool::new(false));
        let discovered_count = Arc::new(AtomicU64::new(0));
        let result = collect_discovered_files(dir.path(), cancelled, discovered_count);
        assert!(result.is_err());
        assert!(
            result
                .err()
                .expect("discovery error")
                .to_string()
                .contains("discovery failed")
        );
    }

    #[tokio::test]
    async fn scan_root_indexes_new_assets_end_to_end() {
        let _guard = ScanTestGuard::new();
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        std::fs::write(
            photos.join("end-to-end.jpg"),
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        let scanner = ScanService::new(catalog.pool().clone(), dir.path().join("thumbs"));
        let summary = scanner
            .scan_root(root.id, &ScanControl::noop())
            .await
            .unwrap();
        assert!(summary.scanned >= 1);
        assert!(summary.indexed >= 1);
        let asset = AssetRepo::new(catalog.pool().clone())
            .find_by_path(root.id, "end-to-end.jpg")
            .await
            .unwrap()
            .unwrap();
        assert!(asset.thumb_key.is_some());
    }

    #[test]
    fn test_hooks_with_env_test_lock_runs_closure() {
        let value = test_hooks::with_env_test_lock(|| 42);
        assert_eq!(value, 42);
        noop_scan_progress(0, 0);
    }

    #[test]
    fn collect_discovered_files_fails_on_strip_prefix_env() {
        let _guard = ScanTestGuard::new();
        std::env::set_var("MEMHG_TEST_STRIP_PREFIX_FAIL", "1");
        let dir = tempdir().unwrap();
        std::fs::write(
            dir.path().join("strip-fail.jpg"),
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();
        let cancelled = Arc::new(AtomicBool::new(false));
        let discovered_count = Arc::new(AtomicU64::new(0));
        let result = collect_discovered_files(dir.path(), cancelled, discovered_count);
        assert!(result.is_err());
        assert!(
            result
                .err()
                .expect("strip prefix error")
                .to_string()
                .contains("strip prefix failed")
        );
    }

    #[cfg(unix)]
    #[test]
    fn collect_discovered_files_fails_on_broken_symlink_metadata() {
        let _guard = ScanTestGuard::new();
        let dir = tempdir().unwrap();
        std::os::unix::fs::symlink(dir.path().join("missing.jpg"), dir.path().join("broken.jpg"))
            .unwrap();
        let cancelled = Arc::new(AtomicBool::new(false));
        let discovered_count = Arc::new(AtomicU64::new(0));
        let result = collect_discovered_files(dir.path(), cancelled, discovered_count);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn scan_inventory_fails_when_discovery_task_panics() {
        let _guard = ScanTestGuard::new();
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        std::fs::write(
            photos.join("one.jpg"),
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        std::env::set_var("MEMHG_TEST_DISCOVERY_PANIC", "1");
        let scanner = ScanService::new(catalog.pool().clone(), dir.path().join("thumbs"));
        let result = scanner
            .scan_inventory(root.id, &ScanControl::noop(), noop_scan_progress)
            .await;
        assert!(result.is_err());
        assert!(
            result
                .err()
                .expect("discovery panic error")
                .to_string()
                .contains("discovery failed")
        );
    }

    #[tokio::test]
    async fn process_index_queue_fails_when_index_batch_panics() {
        let _guard = ScanTestGuard::new();
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let scanner = ScanService::new(catalog.pool().clone(), dir.path().join("thumbs"));
        let item = PostProcessItem {
            asset_id: 1,
            root_id: 1,
            rel_path: "a.jpg".into(),
            path: dir.path().join("a.jpg"),
            mtime_ns: 1,
            kind: "image".into(),
            prior_thumb_key: None,
        };
        std::env::set_var("MEMHG_TEST_INDEX_BATCH_PANIC", "1");
        let err = scanner
            .process_index_queue(&ScanControl::noop(), &[item], noop_scan_progress)
            .await
            .unwrap_err();
        assert!(err.to_string().contains("index batch failed"));
    }

    #[tokio::test]
    async fn scan_inventory_breaks_after_unpause_when_stop_env_set() {
        let _guard = ScanTestGuard::new();
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        for index in 0..3 {
            std::fs::write(
                photos.join(format!("img-{index}.jpg")),
                include_bytes!("../../tests/fixtures/minimal.jpg"),
            )
            .unwrap();
        }
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        let paused = Arc::new(AtomicBool::new(true));
        let cancelled = Arc::new(AtomicBool::new(false));
        let ctrl = ScanControl::new(paused.clone(), cancelled.clone());
        let scanner = ScanService::new(catalog.pool().clone(), dir.path().join("thumbs"));
        let handle = tokio::spawn(async move {
            scanner
                .scan_inventory(root.id, &ctrl, noop_scan_progress)
                .await
        });
        tokio::time::sleep(Duration::from_millis(100)).await;
        std::env::set_var("MEMHG_TEST_CANCEL_AFTER_PAUSE", "1");
        paused.store(false, Ordering::SeqCst);
        handle.await.unwrap().unwrap();
    }

    #[tokio::test]
    async fn scan_service_propagates_errors_when_pool_closed() {
        let _guard = ScanTestGuard::new();
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        std::fs::write(
            photos.join("close.jpg"),
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let pool = catalog.pool().clone();
        let roots = SourceRootRepo::new(pool.clone());
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        let scanner = ScanService::new(pool.clone(), dir.path().join("thumbs"));
        scanner
            .scan_inventory(root.id, &ScanControl::noop(), noop_scan_progress)
            .await
            .unwrap();
        pool.close().await;
        assert!(
            scanner
                .scan_root(root.id, &ScanControl::noop())
                .await
                .is_err()
        );
        assert!(
            scanner
                .scan_inventory(root.id, &ScanControl::noop(), noop_scan_progress)
                .await
                .is_err()
        );
        assert!(
            scanner
                .finalize_scan_links(root.id, &photos)
                .await
                .is_err()
        );
        let item = PostProcessItem {
            asset_id: 1,
            root_id: root.id,
            rel_path: "close.jpg".into(),
            path: photos.join("close.jpg"),
            mtime_ns: 1,
            kind: "image".into(),
            prior_thumb_key: None,
        };
        assert!(
            scanner
                .process_index_queue(&ScanControl::noop(), &[item], noop_scan_progress)
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn scan_root_propagates_index_queue_failure() {
        let _guard = ScanTestGuard::new();
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        std::fs::write(
            photos.join("queue.jpg"),
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        let scanner = ScanService::new(catalog.pool().clone(), dir.path().join("thumbs"));
        std::env::set_var("MEMHG_TEST_INDEX_QUEUE_FAIL", "1");
        let err = scanner
            .scan_root(root.id, &ScanControl::noop())
            .await
            .err()
            .expect("expected index queue failure");
        assert!(err.to_string().contains("index queue failed"));
    }

    #[test]
    fn collect_discovered_files_strip_prefix_map_err() {
        let _guard = ScanTestGuard::new();
        std::env::set_var("MEMHG_TEST_STRIP_PREFIX_MAP_ERR", "1");
        let dir = tempdir().unwrap();
        std::fs::write(
            dir.path().join("strip-map.jpg"),
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();
        let cancelled = Arc::new(AtomicBool::new(false));
        let discovered_count = Arc::new(AtomicU64::new(0));
        let result = collect_discovered_files(dir.path(), cancelled, discovered_count);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn finalize_scan_links_succeeds_for_scanned_root() {
        let _guard = ScanTestGuard::new();
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        std::fs::write(
            photos.join("link.jpg"),
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let roots = SourceRootRepo::new(catalog.pool().clone());
        let root = roots
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        let scanner = ScanService::new(catalog.pool().clone(), dir.path().join("thumbs"));
        scanner
            .scan_inventory(root.id, &ScanControl::noop(), noop_scan_progress)
            .await
            .unwrap();
        scanner
            .finalize_scan_links(root.id, &photos)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn scan_inventory_offline_set_status_fails_when_pool_closed() {
        let _guard = ScanTestGuard::new();
        let dir = tempdir().unwrap();
        let missing = dir.path().join("missing-root");
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let pool = catalog.pool().clone();
        let roots = SourceRootRepo::new(pool.clone());
        let root = roots
            .insert_root(missing.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        pool.close().await;
        let scanner = ScanService::new(pool, dir.path().join("thumbs"));
        assert!(
            scanner
                .scan_inventory(root.id, &ScanControl::noop(), noop_scan_progress)
                .await
                .is_err()
        );
    }
}
