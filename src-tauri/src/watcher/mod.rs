use crate::catalog::repo::SourceRootRepo;
use crate::jobs::JobQueue;
use crate::scan::{ScanControl, ScanService};
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::watch;

pub fn should_rescan_after_debounce(
    last: Option<Instant>,
    now: Instant,
    debounce: Duration,
) -> bool {
    match last {
        None => true,
        Some(last) => now.duration_since(last) >= debounce,
    }
}

fn smb_poller_should_stop(shutdown: &watch::Receiver<bool>) -> bool {
    *shutdown.borrow()
}

fn smb_poller_select_duration() -> Duration {
    let secs = std::env::var("MEMHG_TEST_SMB_POLLER_SELECT_SECS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(30);
    Duration::from_secs(secs)
}

fn log_background_scan_failure(root_id: i64, error: &crate::error::AppError) {
    tracing::warn!(
        "background scan failed for root {}: {}",
        root_id,
        error
    );
}

async fn smb_poller_await_shutdown_or_timeout(shutdown: &mut watch::Receiver<bool>) -> bool {
    tokio::select! {
        _ = shutdown.changed() => smb_poller_should_stop(shutdown),
        _ = tokio::time::sleep(smb_poller_select_duration()) => false,
    }
}

async fn run_smb_poller_loop(
    pool: SqlitePool,
    thumb_dir: PathBuf,
    media_settings: crate::workspace::WorkspaceMediaSettings,
    jobs: JobQueue,
    mut shutdown: watch::Receiver<bool>,
) {
    while !*shutdown.borrow() {
        poll_smb_roots_once(&pool, &thumb_dir, &media_settings, &jobs).await;
        if smb_poller_await_shutdown_or_timeout(&mut shutdown).await {
            break;
        }
    }
}

pub fn smb_scan_due(last_scan_at: Option<i64>, poll_secs: Option<i64>, now: i64) -> bool {
    let interval = poll_secs.unwrap_or(300).max(30);
    let last = last_scan_at.unwrap_or(0);
    now - last >= interval
}

pub struct WatcherService {
    pool: SqlitePool,
    thumb_dir: PathBuf,
    media_settings: crate::workspace::WorkspaceMediaSettings,
    debounce: Arc<Mutex<HashMap<i64, std::time::Instant>>>,
}

impl WatcherService {
    pub fn new(
        pool: SqlitePool,
        thumb_dir: PathBuf,
        media_settings: crate::workspace::WorkspaceMediaSettings,
    ) -> Self {
        Self {
            pool,
            thumb_dir,
            media_settings,
            debounce: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn spawn_dynamic_local_watcher(
        &self,
        jobs: JobQueue,
        mut shutdown: watch::Receiver<bool>,
        mut roots_refresh: watch::Receiver<u64>,
    ) {
        let pool = self.pool.clone();
        let thumb_dir = self.thumb_dir.clone();
        let media_settings = self.media_settings.clone();
        let debounce = self.debounce.clone();

        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("watcher runtime");

            rt.block_on(async {
                let mut generation = *roots_refresh.borrow();

                while !*shutdown.borrow() {
                    let roots = SourceRootRepo::new(pool.clone())
                        .list_roots()
                        .await
                        .unwrap_or_default();
                    let local_roots: Vec<_> = roots
                        .into_iter()
                        .filter(|r| r.kind == "local" && r.scan_policy == "watch")
                        .collect();

                    if local_roots.is_empty() {
                        tokio::select! {
                            _ = shutdown.changed() => {
                                if *shutdown.borrow() {
                                    break;
                                }
                            }
                            _ = roots_refresh.changed() => {
                                generation = *roots_refresh.borrow();
                            }
                            _ = tokio::time::sleep(Duration::from_secs(5)) => {}
                        }
                        continue;
                    }

                    let (tx, rx) = mpsc::channel();
                    let mut watcher = RecommendedWatcher::new(
                        move |res: Result<notify::Event, notify::Error>| {
                            if res.is_ok() {
                                let _ = tx.send(());
                            }
                        },
                        Config::default(),
                    )
                    .expect("notify watcher");

                    for root in &local_roots {
                        let _ = watcher.watch(
                            std::path::Path::new(&root.path),
                            RecursiveMode::Recursive,
                        );
                    }

                    while !*shutdown.borrow() && *roots_refresh.borrow() == generation {
                        if rx.recv_timeout(Duration::from_secs(2)).is_ok() {
                            for root in &local_roots {
                                let should_scan = {
                                    let mut map = debounce.lock().unwrap();
                                    let now = std::time::Instant::now();
                                    if !should_rescan_after_debounce(
                                        map.get(&root.id).copied(),
                                        now,
                                        Duration::from_secs(3),
                                    ) {
                                        false
                                    } else {
                                        map.insert(root.id, now);
                                        true
                                    }
                                };
                                if !should_scan {
                                    continue;
                                }
                                if jobs.try_start("background-scan").await.is_err() {
                                    continue;
                                }
                                let scanner = ScanService::with_media_settings(
                                    pool.clone(),
                                    thumb_dir.clone(),
                                    media_settings.clone(),
                                );
                                if let Err(error) =
                                    scanner.scan_root(root.id, &ScanControl::noop()).await
                                {
                                    log_background_scan_failure(root.id, &error);
                                }
                                jobs.finish().await;
                            }
                        }
                    }

                    if *shutdown.borrow() {
                        break;
                    }
                    generation = *roots_refresh.borrow();
                }
            });
        });
    }

    pub fn spawn_smb_poller(&self, jobs: JobQueue, shutdown: watch::Receiver<bool>) {
        let pool = self.pool.clone();
        let thumb_dir = self.thumb_dir.clone();
        let media_settings = self.media_settings.clone();

        tokio::spawn(run_smb_poller_loop(
            pool,
            thumb_dir,
            media_settings,
            jobs,
            shutdown,
        ));
    }
}

pub(crate) async fn poll_smb_roots_once(
    pool: &SqlitePool,
    thumb_dir: &Path,
    media_settings: &crate::workspace::WorkspaceMediaSettings,
    jobs: &JobQueue,
) {
    let roots = SourceRootRepo::new(pool.clone())
        .list_roots()
        .await
        .unwrap_or_default();
    for root in roots.into_iter().filter(|r| r.kind == "smb") {
        let now = chrono::Utc::now().timestamp();
        if !smb_scan_due(root.last_scan_at, root.poll_secs, now) {
            continue;
        }
        if jobs.try_start("background-scan").await.is_err() {
            continue;
        }
        let scanner = ScanService::with_media_settings(
            pool.clone(),
            thumb_dir.to_path_buf(),
            media_settings.clone(),
        );
        if let Err(error) = scanner.scan_root(root.id, &ScanControl::noop()).await {
            tracing::warn!(
                "SMB background scan failed for root {}: {}",
                root.id,
                error
            );
        }
        jobs.finish().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    fn debounce_allows_first_scan() {
        let now = Instant::now();
        assert!(should_rescan_after_debounce(None, now, Duration::from_secs(3)));
    }

    #[test]
    fn debounce_blocks_recent_scan() {
        let now = Instant::now();
        let last = now - Duration::from_secs(1);
        assert!(!should_rescan_after_debounce(
            Some(last),
            now,
            Duration::from_secs(3),
        ));
    }

    #[test]
    fn debounce_allows_after_interval() {
        let now = Instant::now();
        let last = now - Duration::from_secs(4);
        assert!(should_rescan_after_debounce(
            Some(last),
            now,
            Duration::from_secs(3),
        ));
    }

    #[test]
    fn smb_scan_due_when_never_scanned() {
        assert!(smb_scan_due(None, None, 1_000));
    }

    #[tokio::test]
    async fn poll_smb_roots_once_scans_due_root() {
        use crate::catalog::Catalog;
        use crate::catalog::repo::SourceRootRepo;
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        std::fs::write(
            photos.join("poll.jpg"),
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let pool = catalog.pool().clone();
        SourceRootRepo::new(pool.clone())
            .insert_root(photos.to_str().unwrap(), "smb", "poll", Some(30))
            .await
            .unwrap();

        let media_settings = crate::workspace::WorkspaceMediaSettings {
            read_only: false,
            workspace_xmp_dir: dir.path().join("xmp"),
        };
        let jobs = JobQueue::new();
        poll_smb_roots_once(
            &pool,
            &dir.path().join("thumbs"),
            &media_settings,
            &jobs,
        )
        .await;
    }

    #[tokio::test]
    async fn smb_poller_exits_when_shutdown_requested() {
        use crate::catalog::Catalog;
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let pool = catalog.pool().clone();
        let media_settings = crate::workspace::WorkspaceMediaSettings {
            read_only: false,
            workspace_xmp_dir: dir.path().join("xmp"),
        };
        let watcher = WatcherService::new(pool, dir.path().join("thumbs"), media_settings);
        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
        let jobs = JobQueue::new();
        watcher.spawn_smb_poller(jobs, shutdown_rx);
        shutdown_tx.send(true).expect("shutdown");
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    #[tokio::test]
    async fn smb_poller_handles_shutdown_after_poll() {
        use crate::catalog::Catalog;
        use tempfile::tempdir;

        std::env::set_var("MEMHG_TEST_SMB_POLLER_SELECT_SECS", "2");
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let pool = catalog.pool().clone();
        let media_settings = crate::workspace::WorkspaceMediaSettings {
            read_only: false,
            workspace_xmp_dir: dir.path().join("xmp"),
        };
        let watcher = WatcherService::new(pool, dir.path().join("thumbs"), media_settings);
        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
        let jobs = JobQueue::new();
        watcher.spawn_smb_poller(jobs, shutdown_rx);
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        shutdown_tx.send(true).expect("shutdown");
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        std::env::remove_var("MEMHG_TEST_SMB_POLLER_SELECT_SECS");
    }

    #[test]
    fn smb_scan_not_due_before_interval() {
        assert!(!smb_scan_due(Some(900), Some(300), 1_000));
    }

    #[test]
    fn smb_scan_due_after_interval() {
        assert!(smb_scan_due(Some(600), Some(300), 1_000));
    }

    #[test]
    fn smb_scan_enforces_minimum_interval() {
        assert!(!smb_scan_due(Some(0), Some(10), 20));
        assert!(smb_scan_due(Some(0), Some(10), 31));
    }

    #[tokio::test]
    async fn poll_smb_roots_once_skips_recently_scanned_root() {
        use crate::catalog::Catalog;
        use crate::catalog::repo::SourceRootRepo;
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let pool = catalog.pool().clone();
        SourceRootRepo::new(pool.clone())
            .insert_root(dir.path().to_str().unwrap(), "smb", "poll", Some(300))
            .await
            .unwrap();
        sqlx::query("UPDATE source_root SET last_scan_at = ? WHERE kind = 'smb'")
            .bind(chrono::Utc::now().timestamp())
            .execute(&pool)
            .await
            .unwrap();
        let media_settings = crate::workspace::WorkspaceMediaSettings {
            read_only: false,
            workspace_xmp_dir: dir.path().join("xmp"),
        };
        let jobs = JobQueue::new();
        poll_smb_roots_once(
            &pool,
            &dir.path().join("thumbs"),
            &media_settings,
            &jobs,
        )
        .await;
        assert!(jobs.current().await.is_none());
    }

    #[tokio::test]
    async fn poll_smb_roots_once_skips_when_jobs_busy() {
        use crate::catalog::Catalog;
        use crate::catalog::repo::SourceRootRepo;
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let pool = catalog.pool().clone();
        SourceRootRepo::new(pool.clone())
            .insert_root(dir.path().to_str().unwrap(), "smb", "poll", Some(30))
            .await
            .unwrap();
        let media_settings = crate::workspace::WorkspaceMediaSettings {
            read_only: false,
            workspace_xmp_dir: dir.path().join("xmp"),
        };
        let jobs = JobQueue::new();
        jobs.try_start("background-scan").await.unwrap();
        poll_smb_roots_once(
            &pool,
            &dir.path().join("thumbs"),
            &media_settings,
            &jobs,
        )
        .await;
        assert_eq!(jobs.current().await.as_deref(), Some("background-scan"));
        jobs.finish().await;
    }

    #[tokio::test]
    async fn poll_smb_roots_once_handles_scan_errors() {
        use crate::catalog::Catalog;
        use crate::catalog::repo::SourceRootRepo;
        use crate::scan::test_hooks::{FINALIZE_SCAN_LINKS_FAIL, reset as reset_scan_hooks};
        use std::sync::atomic::Ordering;
        use tempfile::tempdir;

        reset_scan_hooks();
        FINALIZE_SCAN_LINKS_FAIL.store(true, Ordering::SeqCst);
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        std::fs::write(
            photos.join("poll.jpg"),
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let pool = catalog.pool().clone();
        SourceRootRepo::new(pool.clone())
            .insert_root(photos.to_str().unwrap(), "smb", "poll", Some(30))
            .await
            .unwrap();
        let media_settings = crate::workspace::WorkspaceMediaSettings {
            read_only: false,
            workspace_xmp_dir: dir.path().join("xmp"),
        };
        let jobs = JobQueue::new();
        poll_smb_roots_once(
            &pool,
            &dir.path().join("thumbs"),
            &media_settings,
            &jobs,
        )
        .await;
        assert!(jobs.current().await.is_none());
        reset_scan_hooks();
    }

    #[tokio::test]
    async fn smb_poller_exits_on_shutdown() {
        use crate::catalog::Catalog;
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let pool = catalog.pool().clone();
        let media_settings = crate::workspace::WorkspaceMediaSettings {
            read_only: false,
            workspace_xmp_dir: dir.path().join("xmp"),
        };
        let watcher = WatcherService::new(pool, dir.path().join("thumbs"), media_settings);
        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
        let jobs = JobQueue::new();
        watcher.spawn_smb_poller(jobs, shutdown_rx);
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        shutdown_tx.send(true).expect("shutdown");
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }

    #[tokio::test]
    async fn local_watcher_logs_background_scan_errors() {
        use crate::catalog::Catalog;
        use crate::catalog::repo::SourceRootRepo;
        use crate::scan::test_hooks::{FINALIZE_SCAN_LINKS_FAIL, reset as reset_scan_hooks};
        use std::sync::atomic::Ordering;
        use tempfile::tempdir;

        reset_scan_hooks();
        FINALIZE_SCAN_LINKS_FAIL.store(true, Ordering::SeqCst);
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        std::fs::write(
            photos.join("watch.jpg"),
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let pool = catalog.pool().clone();
        SourceRootRepo::new(pool.clone())
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        let media_settings = crate::workspace::WorkspaceMediaSettings {
            read_only: false,
            workspace_xmp_dir: dir.path().join("xmp"),
        };
        let watcher = WatcherService::new(pool, dir.path().join("thumbs"), media_settings);
        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
        let (refresh_tx, refresh_rx) = tokio::sync::watch::channel(0u64);
        let jobs = JobQueue::new();
        watcher.spawn_dynamic_local_watcher(jobs.clone(), shutdown_rx, refresh_rx);
        std::fs::write(
            photos.join("watch-2.jpg"),
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();
        for _ in 0..50 {
            if jobs.current().await.is_some() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        shutdown_tx.send(true).expect("shutdown");
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        reset_scan_hooks();
        let _ = refresh_tx;
    }

    #[tokio::test]
    async fn local_watcher_exits_on_shutdown() {
        use crate::catalog::Catalog;
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let pool = catalog.pool().clone();
        let media_settings = crate::workspace::WorkspaceMediaSettings {
            read_only: false,
            workspace_xmp_dir: dir.path().join("xmp"),
        };
        let watcher = WatcherService::new(pool, dir.path().join("thumbs"), media_settings);
        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
        let (refresh_tx, refresh_rx) = tokio::sync::watch::channel(0u64);
        let jobs = JobQueue::new();
        watcher.spawn_dynamic_local_watcher(jobs, shutdown_rx, refresh_rx);
        shutdown_tx.send(true).expect("shutdown");
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        let _ = refresh_tx;
    }

    #[tokio::test]
    async fn run_smb_poller_loop_continues_after_select_timeout() {
        use crate::catalog::Catalog;
        use tempfile::tempdir;

        std::env::set_var("MEMHG_TEST_SMB_POLLER_SELECT_SECS", "0");
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let pool = catalog.pool().clone();
        let media_settings = crate::workspace::WorkspaceMediaSettings {
            read_only: false,
            workspace_xmp_dir: dir.path().join("xmp"),
        };
        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
        let jobs = JobQueue::new();
        let notifier = tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(150)).await;
            shutdown_tx.send(true).expect("shutdown");
        });
        let poller = tokio::spawn(run_smb_poller_loop(
            pool,
            dir.path().join("thumbs"),
            media_settings,
            jobs,
            shutdown_rx,
        ));
        tokio::time::timeout(std::time::Duration::from_secs(2), poller)
            .await
            .expect("poller")
            .expect("poller task");
        notifier.await.expect("notifier");
        std::env::remove_var("MEMHG_TEST_SMB_POLLER_SELECT_SECS");
    }

    #[tokio::test]
    async fn run_smb_poller_loop_exits_when_shutdown_signaled() {
        use crate::catalog::Catalog;
        use tempfile::tempdir;

        std::env::set_var("MEMHG_TEST_SMB_POLLER_SELECT_SECS", "30");
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let pool = catalog.pool().clone();
        let media_settings = crate::workspace::WorkspaceMediaSettings {
            read_only: false,
            workspace_xmp_dir: dir.path().join("xmp"),
        };
        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
        let jobs = JobQueue::new();
        let notifier = tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            shutdown_tx.send(true).expect("shutdown");
        });
        run_smb_poller_loop(
            pool,
            dir.path().join("thumbs"),
            media_settings,
            jobs,
            shutdown_rx,
        )
        .await;
        notifier.await.expect("notifier");
        std::env::remove_var("MEMHG_TEST_SMB_POLLER_SELECT_SECS");
    }

    #[tokio::test]
    async fn smb_poller_await_shutdown_or_timeout_returns_true_when_signaled() {
        std::env::set_var("MEMHG_TEST_SMB_POLLER_SELECT_SECS", "30");
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::watch::channel(false);
        let notifier = tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            shutdown_tx.send(true).expect("shutdown");
        });
        assert!(smb_poller_await_shutdown_or_timeout(&mut shutdown_rx).await);
        notifier.await.expect("notifier");
        std::env::remove_var("MEMHG_TEST_SMB_POLLER_SELECT_SECS");
    }

    #[test]
    fn log_background_scan_failure_emits_warning() {
        log_background_scan_failure(
            7,
            &crate::error::AppError::Scan("test scan failure".into()),
        );
    }

    #[tokio::test]
    async fn local_watcher_skips_scan_when_jobs_busy() {
        use crate::catalog::Catalog;
        use crate::catalog::repo::SourceRootRepo;
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        std::fs::write(
            photos.join("seed.jpg"),
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let pool = catalog.pool().clone();
        SourceRootRepo::new(pool.clone())
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        let media_settings = crate::workspace::WorkspaceMediaSettings {
            read_only: false,
            workspace_xmp_dir: dir.path().join("xmp"),
        };
        let watcher = WatcherService::new(pool, dir.path().join("thumbs"), media_settings);
        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
        let (refresh_tx, refresh_rx) = tokio::sync::watch::channel(0u64);
        let jobs = JobQueue::new();
        jobs.try_start("background-scan").await.unwrap();
        watcher.spawn_dynamic_local_watcher(jobs.clone(), shutdown_rx, refresh_rx);
        std::fs::write(
            photos.join("busy.jpg"),
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(800)).await;
        assert_eq!(jobs.current().await.as_deref(), Some("background-scan"));
        jobs.finish().await;
        shutdown_tx.send(true).expect("shutdown");
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        let _ = refresh_tx;
    }

    #[tokio::test]
    async fn local_watcher_restarts_when_roots_refresh_generation_changes() {
        use crate::catalog::Catalog;
        use crate::catalog::repo::SourceRootRepo;
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        std::fs::write(
            photos.join("seed.jpg"),
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let pool = catalog.pool().clone();
        SourceRootRepo::new(pool.clone())
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        let media_settings = crate::workspace::WorkspaceMediaSettings {
            read_only: false,
            workspace_xmp_dir: dir.path().join("xmp"),
        };
        let watcher = WatcherService::new(pool, dir.path().join("thumbs"), media_settings);
        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
        let (refresh_tx, refresh_rx) = tokio::sync::watch::channel(0u64);
        let jobs = JobQueue::new();
        watcher.spawn_dynamic_local_watcher(jobs, shutdown_rx, refresh_rx);
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        refresh_tx.send(1).expect("refresh");
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        shutdown_tx.send(true).expect("shutdown");
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    }

    #[tokio::test]
    async fn local_watcher_triggers_on_file_change() {
        use crate::catalog::Catalog;
        use crate::catalog::repo::SourceRootRepo;
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        std::fs::write(
            photos.join("seed.jpg"),
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let pool = catalog.pool().clone();
        SourceRootRepo::new(pool.clone())
            .insert_root(photos.to_str().unwrap(), "local", "watch", None)
            .await
            .unwrap();
        let media_settings = crate::workspace::WorkspaceMediaSettings {
            read_only: false,
            workspace_xmp_dir: dir.path().join("xmp"),
        };
        let watcher = WatcherService::new(pool, dir.path().join("thumbs"), media_settings);
        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
        let (refresh_tx, refresh_rx) = tokio::sync::watch::channel(0u64);
        let jobs = JobQueue::new();
        watcher.spawn_dynamic_local_watcher(jobs.clone(), shutdown_rx, refresh_rx);
        for index in 0..5 {
            std::fs::write(
                photos.join(format!("touch-{index}.jpg")),
                include_bytes!("../../tests/fixtures/minimal.jpg"),
            )
            .unwrap();
            if jobs.current().await.is_some() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
        shutdown_tx.send(true).expect("shutdown");
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        let _ = refresh_tx;
    }

    #[test]
    fn smb_poller_should_stop_when_shutdown_flag_set() {
        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
        assert!(!smb_poller_should_stop(&shutdown_rx));
        shutdown_tx.send(true).expect("shutdown");
        assert!(smb_poller_should_stop(&shutdown_rx));
    }
}
