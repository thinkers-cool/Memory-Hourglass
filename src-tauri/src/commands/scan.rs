use crate::activity::record::record_scan_completed;
use crate::activity::ActivityRecorder;
use crate::commands::context::trace_command;
use crate::error::Result;
use crate::jobs::JobQueue;
use crate::scan::{IndexedThumbUpdate, ScanControl, ScanService};
use crate::state::AppState;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Runtime, State};
use tokio::sync::RwLock;

#[derive(Clone, Serialize)]
pub struct ScanProgressEvent {
    pub root_id: i64,
    pub stage: String,
    pub scanned: u64,
    pub indexed: u64,
}

#[derive(Clone, Serialize)]
pub struct ScanThumbItem {
    pub asset_id: i64,
    pub thumb_path: String,
}

#[derive(Clone, Serialize)]
pub struct ScanThumbsEvent {
    pub root_id: i64,
    pub thumbs: Vec<ScanThumbItem>,
}

pub(crate) struct ScanJobParams<R: Runtime> {
    pub root_id: i64,
    pub correlation_id: String,
    pub app: AppHandle<R>,
    pub scanner: ScanService,
    pub ctrl: ScanControl,
    pub jobs: JobQueue,
    pub scan_status: Arc<RwLock<crate::state::ScanStatus>>,
    pub activity_pool: sqlx::SqlitePool,
}

pub(crate) async fn run_scan_job<R: Runtime>(params: ScanJobParams<R>) {
    let ScanJobParams {
        root_id,
        correlation_id,
        app,
        scanner,
        ctrl,
        jobs,
        scan_status,
        activity_pool,
    } = params;
    let scan_status_final = scan_status.clone();
    let progress_settled = Arc::new(AtomicBool::new(false));

    let emit_progress = |stage: &str, scanned: u64, indexed: u64| {
        let stage_label = stage.to_string();
        let _ = app.emit(
            "scan://progress",
            ScanProgressEvent {
                root_id,
                stage: stage_label.clone(),
                scanned,
                indexed,
            },
        );
        let status = scan_status.clone();
        let settled = progress_settled.clone();
        tauri::async_runtime::spawn(async move {
            update_scan_status_unless_settled(&settled, &status, stage_label, scanned, indexed)
                .await;
        });
    };

    let emit_thumbs = |thumbs: &[IndexedThumbUpdate]| {
        if thumbs.is_empty() {
            return;
        }
        let _ = app.emit(
            "scan://thumbs",
            ScanThumbsEvent {
                root_id,
                thumbs: thumbs
                    .iter()
                    .map(|thumb| ScanThumbItem {
                        asset_id: thumb.asset_id,
                        thumb_path: thumb.thumb_path.clone(),
                    })
                    .collect(),
            },
        );
    };

    let inventory = match scanner
        .scan_root_with_progress(root_id, &ctrl, |scanned, indexed| {
            emit_progress("cataloging", scanned, indexed)
        })
        .await
    {
        Ok(inventory) => inventory,
        Err(error) => {
            tracing::error!(
                root_id,
                correlation_id = %correlation_id,
                error = %error,
                "scan failed"
            );
            settle_scan_progress(&progress_settled, &scan_status_final, "error", 0, 0, false).await;
            emit_progress("error", 0, 0);
            jobs.finish().await;
            return;
        }
    };

    let summary = inventory.summary;
    let index_queue = inventory.index_queue;
    let root_path = inventory.root_path;

    if index_queue.is_empty() {
        if scanner
            .finalize_scan_links(root_id, &root_path)
            .await
            .is_ok()
        {
            emit_progress("done", summary.scanned, summary.indexed);
            let recorder = ActivityRecorder::new(activity_pool.clone());
            let _ =
                record_scan_completed(&recorder, Some(&correlation_id), root_id, &summary).await;
            settle_scan_progress(
                &progress_settled,
                &scan_status_final,
                "done",
                summary.scanned,
                summary.indexed,
                false,
            )
            .await;
        } else {
            settle_scan_progress(
                &progress_settled,
                &scan_status_final,
                "error",
                summary.scanned,
                summary.indexed,
                false,
            )
            .await;
            emit_progress("error", summary.scanned, summary.indexed);
        }
        jobs.finish().await;
        return;
    }

    emit_progress("cataloging", summary.scanned, summary.indexed);

    if scanner
        .process_index_queue(&ctrl, &index_queue, |indexed, total, thumbs| {
            emit_progress("indexing", total, indexed);
            emit_thumbs(thumbs);
        })
        .await
        .is_err()
    {
        settle_scan_progress(
            &progress_settled,
            &scan_status_final,
            "error",
            summary.scanned,
            summary.indexed,
            false,
        )
        .await;
        emit_progress("error", summary.scanned, summary.indexed);
        jobs.finish().await;
        return;
    }

    if scanner
        .finalize_scan_links(root_id, &root_path)
        .await
        .is_ok()
    {
        emit_progress("done", summary.scanned, summary.indexed);
        let recorder = ActivityRecorder::new(activity_pool.clone());
        let _ = record_scan_completed(&recorder, Some(&correlation_id), root_id, &summary).await;
        settle_scan_progress(
            &progress_settled,
            &scan_status_final,
            "done",
            summary.scanned,
            summary.indexed,
            false,
        )
        .await;
    } else {
        settle_scan_progress(
            &progress_settled,
            &scan_status_final,
            "error",
            summary.scanned,
            summary.indexed,
            false,
        )
        .await;
        emit_progress("error", summary.scanned, summary.indexed);
    }

    jobs.finish().await;
}

async fn update_scan_status_unless_settled(
    settled: &AtomicBool,
    status: &RwLock<crate::state::ScanStatus>,
    stage: String,
    scanned: u64,
    indexed: u64,
) {
    if settled.load(Ordering::SeqCst) {
        return;
    }
    let mut current = status.write().await;
    if settled.load(Ordering::SeqCst) {
        return;
    }
    current.stage = stage;
    current.scanned = scanned;
    current.indexed = indexed;
}

async fn settle_scan_progress(
    progress_settled: &AtomicBool,
    scan_status: &Arc<RwLock<crate::state::ScanStatus>>,
    stage: &str,
    scanned: u64,
    indexed: u64,
    running: bool,
) {
    progress_settled.store(true, Ordering::SeqCst);
    let mut status = scan_status.write().await;
    status.running = running;
    status.stage = stage.into();
    status.scanned = scanned;
    status.indexed = indexed;
}

#[tauri::command]
pub async fn start_scan<R: Runtime>(
    root_id: i64,
    app: AppHandle<R>,
    state: State<'_, AppState>,
) -> Result<()> {
    trace_command("start_scan", |correlation_id| async move {
        let (scanner, ctrl, jobs, scan_status, activity_pool) = state
            .with_active(|ws| async move {
                ws.jobs.try_start("scan").await?;
                ws.resume_scan();
                {
                    let mut status = ws.scan_status.write().await;
                    status.root_id = Some(root_id);
                    status.stage = "cataloging".into();
                    status.running = true;
                    status.scanned = 0;
                    status.indexed = 0;
                }
                Ok((
                    ws.scan_service(),
                    ws.scan_control(),
                    ws.jobs.clone(),
                    ws.scan_status.clone(),
                    ws.catalog.pool().clone(),
                ))
            })
            .await?;

        tauri::async_runtime::spawn(run_scan_job(ScanJobParams {
            root_id,
            correlation_id,
            app,
            scanner,
            ctrl,
            jobs,
            scan_status,
            activity_pool,
        }));

        Ok(())
    })
    .await
}

#[tauri::command]
pub async fn pause_scan(state: State<'_, AppState>) -> Result<()> {
    trace_command("pause_scan", |_correlation_id| async move {
        state
            .with_active(|ws| async move {
                ws.pause_scan();
                Ok(())
            })
            .await
    })
    .await
}

#[tauri::command]
pub async fn resume_scan(state: State<'_, AppState>) -> Result<()> {
    trace_command("resume_scan", |_correlation_id| async move {
        state
            .with_active(|ws| async move {
                ws.resume_scan();
                Ok(())
            })
            .await
    })
    .await
}

#[tauri::command]
pub async fn cancel_scan(state: State<'_, AppState>) -> Result<()> {
    trace_command("cancel_scan", |_correlation_id| async move {
        state
            .with_active(|ws| async move {
                ws.jobs.request_cancel();
                Ok(())
            })
            .await
    })
    .await
}

#[tauri::command]
pub async fn get_scan_status(state: State<'_, AppState>) -> Result<ScanProgressEvent> {
    trace_command("get_scan_status", |_correlation_id| async move {
        state
            .with_active(|ws| async move {
                let status = ws.scan_status.read().await;
                Ok(scan_progress_from_status(&status))
            })
            .await
    })
    .await
}

fn scan_progress_from_status(status: &crate::state::ScanStatus) -> ScanProgressEvent {
    ScanProgressEvent {
        root_id: status.root_id.unwrap_or(0),
        stage: status.stage.clone(),
        scanned: status.scanned,
        indexed: status.indexed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scan::test_hooks::{reset_unlocked as reset_scan_hooks, FINALIZE_SCAN_LINKS_FAIL};
    use crate::state::AppState;
    use std::sync::atomic::Ordering;
    use tauri::Manager;
    use tempfile::tempdir;

    struct ScanHookGuard {
        _lock: std::sync::MutexGuard<'static, ()>,
    }

    impl ScanHookGuard {
        fn new() -> Self {
            let lock = crate::scan::test_hooks::env_test_guard();
            reset_scan_hooks();
            Self { _lock: lock }
        }
    }

    impl Drop for ScanHookGuard {
        fn drop(&mut self) {
            reset_scan_hooks();
        }
    }

    #[tokio::test]
    async fn get_scan_status_maps_lock_fields() {
        let (state, _guard) = AppState::test_with_fresh_workspace().await.unwrap();
        state
            .with_active(|ws| async move {
                {
                    let mut status = ws.scan_status.write().await;
                    status.root_id = Some(9);
                    status.stage = "scanning".into();
                    status.scanned = 12;
                    status.indexed = 7;
                }
                let status = ws.scan_status.read().await;
                let payload = scan_progress_from_status(&status);
                assert_eq!(payload.root_id, 9);
                assert_eq!(payload.stage, "scanning");
                assert_eq!(payload.scanned, 12);
                assert_eq!(payload.indexed, 7);
                Ok(())
            })
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn pause_resume_and_cancel_update_scan_control() {
        let (state, _guard) = AppState::test_with_fresh_workspace().await.unwrap();
        state
            .with_active(|ws| async move {
                ws.pause_scan();
                assert!(ws.scan_pause.load(Ordering::SeqCst));
                ws.resume_scan();
                assert!(!ws.scan_pause.load(Ordering::SeqCst));
                ws.jobs.request_cancel();
                assert!(ws.scan_control().is_cancelled());
                Ok(())
            })
            .await
            .unwrap();
    }

    async fn scan_job_fixture() -> tauri::App<tauri::test::MockRuntime> {
        let dir = tempdir().unwrap();
        let state = AppState::new(dir.path().join("app")).unwrap();
        let workspace = dir.path().join("ws");
        std::fs::create_dir_all(&workspace).unwrap();
        crate::workspace::init_workspace_at(&workspace, false).unwrap();
        state.open_workspace(&workspace).await.unwrap();
        tauri::test::mock_builder()
            .manage(state)
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .expect("app")
    }

    #[tokio::test]
    async fn run_scan_job_handles_missing_root() {
        let _guard = ScanHookGuard::new();
        let app = scan_job_fixture().await;
        let state = app.state::<AppState>();
        let handle = app.handle().clone();
        state
            .with_active(|ws| async move {
                let jobs = ws.jobs.clone();
                run_scan_job(ScanJobParams {
                    root_id: 999_999,
                    correlation_id: "corr".into(),
                    app: handle,
                    scanner: ws.scan_service(),
                    ctrl: ws.scan_control(),
                    jobs,
                    scan_status: ws.scan_status.clone(),
                    activity_pool: ws.catalog.pool().clone(),
                })
                .await;
                let status = ws.scan_status.read().await;
                assert_eq!(status.stage, "error");
                Ok(())
            })
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn run_scan_job_handles_finalize_failure_on_empty_queue() {
        let _guard = ScanHookGuard::new();
        let app = scan_job_fixture().await;
        let state = app.state::<AppState>();
        let handle = app.handle().clone();
        let photos = tempdir().unwrap().path().join("empty-scan");
        std::fs::create_dir_all(&photos).unwrap();
        let root_id = state
            .with_active(|ws| async move {
                ws.library
                    .add_local_root(photos.to_str().unwrap())
                    .await
                    .map(|root| root.id)
            })
            .await
            .unwrap();
        FINALIZE_SCAN_LINKS_FAIL.store(true, Ordering::SeqCst);
        state
            .with_active(|ws| async move {
                let jobs = ws.jobs.clone();
                run_scan_job(ScanJobParams {
                    root_id,
                    correlation_id: "corr".into(),
                    app: handle,
                    scanner: ws.scan_service(),
                    ctrl: ws.scan_control(),
                    jobs,
                    scan_status: ws.scan_status.clone(),
                    activity_pool: ws.catalog.pool().clone(),
                })
                .await;
                let status = ws.scan_status.read().await;
                assert_eq!(status.stage, "error");
                Ok(())
            })
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn run_scan_job_handles_index_queue_failure() {
        let _guard = ScanHookGuard::new();
        let app = scan_job_fixture().await;
        let state = app.state::<AppState>();
        let handle = app.handle().clone();
        let dir = tempdir().unwrap();
        let photos = dir.path().join("indexed");
        std::fs::create_dir_all(&photos).unwrap();
        std::fs::write(
            photos.join("one.jpg"),
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();
        let root_id = state
            .with_active(|ws| async move {
                ws.library
                    .add_local_root(photos.to_str().unwrap())
                    .await
                    .map(|root| root.id)
            })
            .await
            .unwrap();
        std::env::set_var("MEMHG_TEST_INDEX_QUEUE_FAIL", "1");
        state
            .with_active(|ws| async move {
                let jobs = ws.jobs.clone();
                run_scan_job(ScanJobParams {
                    root_id,
                    correlation_id: "corr".into(),
                    app: handle,
                    scanner: ws.scan_service(),
                    ctrl: ws.scan_control(),
                    jobs,
                    scan_status: ws.scan_status.clone(),
                    activity_pool: ws.catalog.pool().clone(),
                })
                .await;
                let status = ws.scan_status.read().await;
                assert_eq!(status.stage, "error");
                Ok(())
            })
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn run_scan_job_completes_with_assets() {
        let _guard = ScanHookGuard::new();
        let app = scan_job_fixture().await;
        let state = app.state::<AppState>();
        let handle = app.handle().clone();
        let dir = tempdir().unwrap();
        let photos = dir.path().join("done");
        std::fs::create_dir_all(&photos).unwrap();
        std::fs::write(
            photos.join("done.jpg"),
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();
        let root_id = state
            .with_active(|ws| async move {
                ws.library
                    .add_local_root(photos.to_str().unwrap())
                    .await
                    .map(|root| root.id)
            })
            .await
            .unwrap();
        state
            .with_active(|ws| async move {
                let jobs = ws.jobs.clone();
                run_scan_job(ScanJobParams {
                    root_id,
                    correlation_id: "corr".into(),
                    app: handle,
                    scanner: ws.scan_service(),
                    ctrl: ws.scan_control(),
                    jobs,
                    scan_status: ws.scan_status.clone(),
                    activity_pool: ws.catalog.pool().clone(),
                })
                .await;
                let status = ws.scan_status.read().await;
                assert_eq!(status.stage, "done");
                Ok(())
            })
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn run_scan_job_handles_finalize_failure_after_indexing() {
        let _guard = ScanHookGuard::new();
        let app = scan_job_fixture().await;
        let state = app.state::<AppState>();
        let handle = app.handle().clone();
        let dir = tempdir().unwrap();
        let photos = dir.path().join("finalize-fail");
        std::fs::create_dir_all(&photos).unwrap();
        std::fs::write(
            photos.join("photo.jpg"),
            include_bytes!("../../tests/fixtures/minimal.jpg"),
        )
        .unwrap();
        let root_id = state
            .with_active(|ws| async move {
                ws.library
                    .add_local_root(photos.to_str().unwrap())
                    .await
                    .map(|root| root.id)
            })
            .await
            .unwrap();
        FINALIZE_SCAN_LINKS_FAIL.store(true, Ordering::SeqCst);
        state
            .with_active(|ws| async move {
                let jobs = ws.jobs.clone();
                run_scan_job(ScanJobParams {
                    root_id,
                    correlation_id: "corr".into(),
                    app: handle,
                    scanner: ws.scan_service(),
                    ctrl: ws.scan_control(),
                    jobs,
                    scan_status: ws.scan_status.clone(),
                    activity_pool: ws.catalog.pool().clone(),
                })
                .await;
                let status = ws.scan_status.read().await;
                assert_eq!(status.stage, "error");
                Ok(())
            })
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn scan_commands_cover_pause_resume_cancel_and_status() {
        let _guard = ScanHookGuard::new();
        let app = scan_job_fixture().await;
        let state = app.state::<AppState>();
        pause_scan(state.clone()).await.unwrap();
        resume_scan(state.clone()).await.unwrap();
        cancel_scan(state.clone()).await.unwrap();
        let status = get_scan_status(state.clone()).await.unwrap();
        assert_eq!(status.stage, "idle");
    }

    #[tokio::test]
    async fn emit_progress_skips_status_after_scan_settled() {
        let _guard = ScanHookGuard::new();
        let app = scan_job_fixture().await;
        let state = app.state::<AppState>();
        let handle = app.handle().clone();
        let photos = tempdir().unwrap().path().join("settled");
        std::fs::create_dir_all(&photos).unwrap();
        let root_id = state
            .with_active(|ws| async move {
                ws.library
                    .add_local_root(photos.to_str().unwrap())
                    .await
                    .map(|root| root.id)
            })
            .await
            .unwrap();
        state
            .with_active(|ws| async move {
                let jobs = ws.jobs.clone();
                run_scan_job(ScanJobParams {
                    root_id,
                    correlation_id: "corr".into(),
                    app: handle,
                    scanner: ws.scan_service(),
                    ctrl: ws.scan_control(),
                    jobs,
                    scan_status: ws.scan_status.clone(),
                    activity_pool: ws.catalog.pool().clone(),
                })
                .await;
                let status = ws.scan_status.read().await;
                assert_eq!(status.stage, "done");
                Ok(())
            })
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn start_scan_command_starts_background_job() {
        let _guard = ScanHookGuard::new();
        let app = scan_job_fixture().await;
        let state = app.state::<AppState>();
        let photos = tempdir().unwrap().path().join("cmd-scan");
        std::fs::create_dir_all(&photos).unwrap();
        let root_id = state
            .with_active(|ws| async move {
                ws.library
                    .add_local_root(photos.to_str().unwrap())
                    .await
                    .map(|root| root.id)
            })
            .await
            .unwrap();
        let app_state = app.state::<AppState>();
        start_scan(root_id, app.handle().clone(), app_state.clone())
            .await
            .unwrap();
        for _ in 0..100 {
            let status = get_scan_status(app_state.clone()).await.unwrap();
            if status.stage == "done" || status.stage == "error" {
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
        panic!("scan did not finish");
    }

    #[tokio::test]
    async fn update_scan_status_skips_when_settled_before_write_lock() {
        use super::update_scan_status_unless_settled;
        use std::sync::Arc;

        let settled = Arc::new(AtomicBool::new(false));
        let status = Arc::new(RwLock::new(crate::state::ScanStatus {
            root_id: None,
            stage: "idle".into(),
            scanned: 0,
            indexed: 0,
            running: false,
        }));
        let write_guard = status.write().await;
        let settled_clone = settled.clone();
        let status_clone = status.clone();
        let task = tokio::spawn(async move {
            update_scan_status_unless_settled(
                &settled_clone,
                &status_clone,
                "cataloging".into(),
                3,
                1,
            )
            .await;
        });
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        settled.store(true, Ordering::SeqCst);
        drop(write_guard);
        task.await.unwrap();
        assert_eq!(status.read().await.stage, "idle");
    }
}
