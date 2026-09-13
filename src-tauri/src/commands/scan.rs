use crate::activity::record::record_scan_completed;
use crate::activity::ActivityRecorder;
use crate::commands::context::{trace_command, trace_command_with_id};
use crate::error::Result;
use crate::jobs::JobPool;
use crate::catalog::repo::SourceRootRepo;
use crate::scan::{IndexedThumbUpdate, ScanControl, ScanService, list_roots_needing_scan};
use crate::state::{AppState, RootScanStatus, ScanStatusMap};
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tauri::{AppHandle, Emitter, Manager, Runtime, State};

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
    pub jobs: JobPool,
    pub cancel: Arc<AtomicBool>,
    pub scan_status: ScanStatusMap,
    pub activity_pools: crate::catalog::pools::CatalogPools,
}

pub(crate) async fn run_scan_job<R: Runtime>(params: ScanJobParams<R>) {
    let ScanJobParams {
        root_id,
        correlation_id,
        app,
        scanner,
        ctrl,
        jobs,
        cancel: _cancel,
        scan_status,
        activity_pools,
    } = params;
    let scan_status_final = scan_status.clone();
    let progress_settled = Arc::new(AtomicBool::new(false));
    let scan_job_start = Instant::now();

    tracing::info!(
        root_id,
        correlation_id = %correlation_id,
        "scan job start"
    );

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
            update_scan_status_unless_settled(
                &settled,
                &status,
                root_id,
                stage_label,
                scanned,
                indexed,
            )
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

    let root = match SourceRootRepo::new(scanner.pools.clone()).get_root(root_id).await {
        Ok(root) => root,
        Err(error) => {
            tracing::error!(
                root_id,
                correlation_id = %correlation_id,
                error = %error,
                "scan failed loading root"
            );
            settle_scan_progress(
                &progress_settled,
                &scan_status_final,
                root_id,
                "error",
                0,
                0,
                false,
            )
            .await;
            emit_progress("error", 0, 0);
            tracing::info!(
                root_id,
                correlation_id = %correlation_id,
                elapsed_ms = scan_job_start.elapsed().as_millis() as u64,
                outcome = "error",
                "scan job complete"
            );
            jobs.finish_scan(root_id).await;
            spawn_pending_scans(app.clone());
            return;
        }
    };
    let scanner = scanner.for_root(&root);

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
            settle_scan_progress(
                &progress_settled,
                &scan_status_final,
                root_id,
                "error",
                0,
                0,
                false,
            )
            .await;
            emit_progress("error", 0, 0);
            tracing::info!(
                root_id,
                correlation_id = %correlation_id,
                elapsed_ms = scan_job_start.elapsed().as_millis() as u64,
                outcome = "error",
                "scan job complete"
            );
            jobs.finish_scan(root_id).await;
            spawn_pending_scans(app.clone());
            return;
        }
    };

    let summary = inventory.summary;
    let index_queue = inventory.index_queue;

    if index_queue.is_empty() {
        let outcome = if scanner
            .finalize_scan_links(root_id)
            .await
            .is_ok()
        {
            scanner.spawn_duplicate_index(root_id);
            emit_progress("done", summary.scanned, summary.indexed);
            let recorder = ActivityRecorder::new(activity_pools.clone());
            let _ =
                record_scan_completed(&recorder, Some(&correlation_id), root_id, &summary).await;
            settle_scan_progress(
                &progress_settled,
                &scan_status_final,
                root_id,
                "done",
                summary.scanned,
                summary.indexed,
                false,
            )
            .await;
            "done"
        } else {
            settle_scan_progress(
                &progress_settled,
                &scan_status_final,
                root_id,
                "error",
                summary.scanned,
                summary.indexed,
                false,
            )
            .await;
            emit_progress("error", summary.scanned, summary.indexed);
            "error"
        };
        tracing::info!(
            root_id,
            correlation_id = %correlation_id,
            elapsed_ms = scan_job_start.elapsed().as_millis() as u64,
            scanned = summary.scanned,
            outcome,
            "scan job complete"
        );
        jobs.finish_scan(root_id).await;
        spawn_pending_scans(app.clone());
        return;
    }

    emit_progress("cataloging", summary.scanned, summary.indexed);

    if scanner
        .process_index_queue(
            &ctrl,
            &index_queue,
            |indexed, total, thumbs| {
                emit_progress("indexing", total, indexed);
                emit_thumbs(thumbs);
            },
        )
        .await
        .is_err()
    {
        settle_scan_progress(
            &progress_settled,
            &scan_status_final,
            root_id,
            "error",
            summary.scanned,
            summary.indexed,
            false,
        )
        .await;
        emit_progress("error", summary.scanned, summary.indexed);
        tracing::info!(
            root_id,
            correlation_id = %correlation_id,
            elapsed_ms = scan_job_start.elapsed().as_millis() as u64,
            scanned = summary.scanned,
            outcome = "error",
            "scan job complete"
        );
        jobs.finish_scan(root_id).await;
        spawn_pending_scans(app.clone());
        return;
    }

    let outcome = if scanner
        .finalize_scan_links(root_id)
        .await
        .is_ok()
    {
        scanner.spawn_duplicate_index(root_id);
        emit_progress("done", summary.scanned, summary.indexed);
        let recorder = ActivityRecorder::new(activity_pools.clone());
        let _ = record_scan_completed(&recorder, Some(&correlation_id), root_id, &summary).await;
        settle_scan_progress(
            &progress_settled,
            &scan_status_final,
            root_id,
            "done",
            summary.scanned,
            summary.indexed,
            false,
        )
        .await;
        "done"
    } else {
        settle_scan_progress(
            &progress_settled,
            &scan_status_final,
            root_id,
            "error",
            summary.scanned,
            summary.indexed,
            false,
        )
        .await;
        emit_progress("error", summary.scanned, summary.indexed);
        "error"
    };

    tracing::info!(
        root_id,
        correlation_id = %correlation_id,
        elapsed_ms = scan_job_start.elapsed().as_millis() as u64,
        scanned = summary.scanned,
        indexed = summary.indexed,
        index_queue = index_queue.len(),
        outcome,
        "scan job complete"
    );

    jobs.finish_scan(root_id).await;
    spawn_pending_scans(app.clone());
}

async fn update_scan_status_unless_settled(
    settled: &AtomicBool,
    status: &ScanStatusMap,
    root_id: i64,
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
    let entry = current.entry(root_id).or_default();
    entry.stage = stage;
    entry.scanned = scanned;
    entry.indexed = indexed;
}

async fn settle_scan_progress(
    progress_settled: &AtomicBool,
    scan_status: &ScanStatusMap,
    root_id: i64,
    stage: &str,
    scanned: u64,
    indexed: u64,
    running: bool,
) {
    progress_settled.store(true, Ordering::SeqCst);
    let mut status = scan_status.write().await;
    let entry = status.entry(root_id).or_default();
    entry.running = running;
    entry.stage = stage.into();
    entry.scanned = scanned;
    entry.indexed = indexed;
}

async fn mark_scan_running(scan_status: &ScanStatusMap, root_id: i64) {
    let mut status = scan_status.write().await;
    status.insert(
        root_id,
        RootScanStatus {
            stage: "cataloging".into(),
            scanned: 0,
            indexed: 0,
            running: true,
        },
    );
}

fn idle_progress(root_id: i64) -> ScanProgressEvent {
    ScanProgressEvent {
        root_id,
        stage: "idle".into(),
        scanned: 0,
        indexed: 0,
    }
}

fn progress_from_root_status(root_id: i64, status: &RootScanStatus) -> ScanProgressEvent {
    ScanProgressEvent {
        root_id,
        stage: status.stage.clone(),
        scanned: status.scanned,
        indexed: status.indexed,
    }
}

pub(crate) async fn schedule_scan_job<R: Runtime>(
    root_id: i64,
    correlation_id: String,
    app: AppHandle<R>,
    state: State<'_, AppState>,
) -> Result<()> {
    if state
        .with_active(|ws| async move { Ok(ws.jobs.is_scan_running(root_id).await) })
        .await?
    {
        return Ok(());
    }

    let started = state
        .with_active(|ws| async move {
            match ws.jobs.try_start_scan(root_id).await {
                Ok(cancel) => {
                    ws.resume_scan();
                    mark_scan_running(&ws.scan_status, root_id).await;
                    Ok(Some((
                        ws.scan_service(),
                        ws.scan_control(cancel.clone()),
                        ws.jobs.clone(),
                        cancel,
                        ws.scan_status.clone(),
                        ws.catalog.pools().clone(),
                    )))
                }
                Err(_) => {
                    ws.jobs.enqueue_scan(root_id).await;
                    Ok(None)
                }
            }
        })
        .await?;

    if let Some((scanner, ctrl, jobs, cancel, scan_status, activity_pools)) = started {
        tauri::async_runtime::spawn(run_scan_job(ScanJobParams {
            root_id,
            correlation_id,
            app,
            scanner,
            ctrl,
            jobs,
            cancel,
            scan_status,
            activity_pools,
        }));
    }

    Ok(())
}

fn spawn_pending_scans<R: Runtime>(app: AppHandle<R>) {
    tauri::async_runtime::spawn(async move {
        let state = app.state::<AppState>();
        while let Ok(Some(root_id)) = state
            .with_active(|ws| async move { Ok(ws.jobs.dequeue_pending_scan().await) })
            .await
        {
            if schedule_scan_job(root_id, "pending".into(), app.clone(), state.clone())
                .await
                .is_err()
            {
                break;
            }
        }
    });
}

#[tauri::command]
pub async fn start_scan<R: Runtime>(
    root_id: i64,
    app: AppHandle<R>,
    state: State<'_, AppState>,
) -> Result<()> {
    trace_command_with_id("start_scan", |correlation_id| async move {
        schedule_scan_job(root_id, correlation_id, app, state).await
    })
    .await
}

#[tauri::command]
pub async fn resume_pending_scans<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
) -> Result<()> {
    trace_command_with_id("resume_pending_scans", |correlation_id| async move {
        let root_ids = state
            .with_active(|ws| async move {
                list_roots_needing_scan(ws.catalog.pools()).await
            })
            .await?;
        for root_id in root_ids {
            schedule_scan_job(root_id, correlation_id.clone(), app.clone(), state.clone())
                .await?;
        }
        Ok(())
    })
    .await
}

#[tauri::command]
pub async fn pause_scan(state: State<'_, AppState>) -> Result<()> {
    trace_command("pause_scan", || async move {
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
    trace_command("resume_scan", || async move {
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
pub async fn cancel_scan(root_id: Option<i64>, state: State<'_, AppState>) -> Result<()> {
    trace_command("cancel_scan", || async move {
        state
            .with_active(|ws| async move {
                if let Some(root_id) = root_id {
                    ws.jobs.request_cancel_scan(root_id).await;
                } else {
                    ws.jobs.request_cancel_all_scans().await;
                }
                Ok(())
            })
            .await
    })
    .await
}

#[tauri::command]
pub async fn get_scan_status(
    root_id: Option<i64>,
    state: State<'_, AppState>,
) -> Result<ScanProgressEvent> {
    trace_command("get_scan_status", || async move {
        state
            .with_active(|ws| async move {
                let status = ws.scan_status.read().await;
                if let Some(root_id) = root_id {
                    return Ok(status
                        .get(&root_id)
                        .map(|entry| progress_from_root_status(root_id, entry))
                        .unwrap_or_else(|| idle_progress(root_id)));
                }
                for (id, entry) in status.iter() {
                    if entry.running {
                        return Ok(progress_from_root_status(*id, entry));
                    }
                }
                Ok(idle_progress(0))
            })
            .await
    })
    .await
}

#[tauri::command]
pub async fn list_scan_statuses(state: State<'_, AppState>) -> Result<Vec<ScanProgressEvent>> {
    trace_command("list_scan_statuses", || async move {
        state
            .with_active(|ws| async move {
                let status = ws.scan_status.read().await;
                let mut events: Vec<ScanProgressEvent> = status
                    .iter()
                    .filter(|(_, entry)| entry.running)
                    .map(|(id, entry)| progress_from_root_status(*id, entry))
                    .collect();
                events.sort_by_key(|event| event.root_id);
                Ok(events)
            })
            .await
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scan::test_hooks::{reset_unlocked as reset_scan_hooks, set_finalize_scan_links_fail};
    use crate::state::AppState;
    use std::sync::atomic::Ordering;
    use tauri::Manager;
    use tempfile::tempdir;

    struct ScanHookGuard {
        #[allow(dead_code)]
        lock: std::sync::MutexGuard<'static, ()>,
    }

    impl ScanHookGuard {
        fn new() -> Self {
            let lock = crate::scan::test_hooks::env_test_guard();
            reset_scan_hooks();
            Self { lock }
        }
    }

    impl Drop for ScanHookGuard {
        fn drop(&mut self) {
            reset_scan_hooks();
        }
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

    async fn run_fixture_scan_job(
        app: &tauri::App<tauri::test::MockRuntime>,
        root_id: i64,
    ) -> i64 {
        let state = app.state::<AppState>();
        let handle = app.handle().clone();
        state
            .with_active(|ws| async move {
                let cancel = ws.jobs.try_start_scan(root_id).await.unwrap();
                run_scan_job(ScanJobParams {
                    root_id,
                    correlation_id: "corr".into(),
                    app: handle,
                    scanner: ws.scan_service(),
                    ctrl: ws.scan_control(cancel.clone()),
                    jobs: ws.jobs.clone(),
                    cancel,
                    scan_status: ws.scan_status.clone(),
                    activity_pools: ws.catalog.pools().clone(),
                })
                .await;
                Ok(root_id)
            })
            .await
            .unwrap()
    }

    async fn root_stage(state: &State<'_, AppState>, root_id: i64) -> String {
        let status = state
            .with_active(|ws| async move { Ok(ws.scan_status.read().await.clone()) })
            .await
            .unwrap();
        status
            .get(&root_id)
            .map(|entry| entry.stage.clone())
            .unwrap_or_else(|| "idle".into())
    }

    #[tokio::test]
    async fn get_scan_status_maps_root_fields() {
        let app = scan_job_fixture().await;
        let state = app.state::<AppState>();
        state
            .with_active(|ws| async move {
                let mut status = ws.scan_status.write().await;
                status.insert(
                    9,
                    RootScanStatus {
                        stage: "scanning".into(),
                        scanned: 12,
                        indexed: 7,
                        running: true,
                    },
                );
                Ok(())
            })
            .await
            .unwrap();
        let payload = get_scan_status(Some(9), state.clone()).await.unwrap();
        assert_eq!(payload.root_id, 9);
        assert_eq!(payload.stage, "scanning");
        assert_eq!(payload.scanned, 12);
        assert_eq!(payload.indexed, 7);
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
                let cancel = ws.jobs.try_start_scan(1).await.unwrap();
                ws.jobs.request_cancel_scan(1).await;
                assert!(cancel.load(Ordering::SeqCst));
                Ok(())
            })
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn run_scan_job_handles_missing_root() {
        let _guard = ScanHookGuard::new();
        let app = scan_job_fixture().await;
        let state = app.state::<AppState>();
        run_fixture_scan_job(&app, 999_999).await;
        assert_eq!(root_stage(&state, 999_999).await, "error");
    }

    #[tokio::test]
    async fn run_scan_job_handles_finalize_failure_on_empty_queue() {
        let _guard = ScanHookGuard::new();
        let app = scan_job_fixture().await;
        let state = app.state::<AppState>();
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
        set_finalize_scan_links_fail(true);
        run_fixture_scan_job(&app, root_id).await;
        assert_eq!(root_stage(&state, root_id).await, "error");
    }

    #[tokio::test]
    async fn run_scan_job_handles_index_queue_failure() {
        let _guard = ScanHookGuard::new();
        let app = scan_job_fixture().await;
        let state = app.state::<AppState>();
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
        crate::scan::test_hooks::set_flag("MEMHG_TEST_INDEX_QUEUE_FAIL");
        run_fixture_scan_job(&app, root_id).await;
        assert_eq!(root_stage(&state, root_id).await, "error");
    }

    #[tokio::test]
    async fn run_scan_job_completes_with_assets() {
        let _guard = ScanHookGuard::new();
        let app = scan_job_fixture().await;
        let state = app.state::<AppState>();
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
        run_fixture_scan_job(&app, root_id).await;
        assert_eq!(root_stage(&state, root_id).await, "done");
    }

    #[tokio::test]
    async fn run_scan_job_handles_finalize_failure_after_indexing() {
        let _guard = ScanHookGuard::new();
        let app = scan_job_fixture().await;
        let state = app.state::<AppState>();
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
        set_finalize_scan_links_fail(true);
        run_fixture_scan_job(&app, root_id).await;
        assert_eq!(root_stage(&state, root_id).await, "error");
    }

    #[tokio::test]
    async fn scan_commands_cover_pause_resume_cancel_and_status() {
        let _guard = ScanHookGuard::new();
        let app = scan_job_fixture().await;
        let state = app.state::<AppState>();
        pause_scan(state.clone()).await.unwrap();
        resume_scan(state.clone()).await.unwrap();
        cancel_scan(None, state.clone()).await.unwrap();
        let status = get_scan_status(None, state.clone()).await.unwrap();
        assert_eq!(status.stage, "idle");
    }

    #[tokio::test]
    async fn emit_progress_skips_status_after_scan_settled() {
        let _guard = ScanHookGuard::new();
        let app = scan_job_fixture().await;
        let state = app.state::<AppState>();
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
        run_fixture_scan_job(&app, root_id).await;
        assert_eq!(root_stage(&state, root_id).await, "done");
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
            let status = get_scan_status(Some(root_id), app_state.clone()).await.unwrap();
            if status.stage == "done" || status.stage == "error" {
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
        panic!("scan did not finish");
    }

    #[tokio::test]
    async fn update_scan_status_skips_when_settled_before_write_lock() {
        let settled = Arc::new(AtomicBool::new(false));
        let status: ScanStatusMap =
            Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new()));
        let write_guard = status.write().await;
        let settled_clone = settled.clone();
        let status_clone = status.clone();
        let task = tokio::spawn(async move {
            update_scan_status_unless_settled(
                &settled_clone,
                &status_clone,
                1,
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
        assert!(!status.read().await.contains_key(&1));
    }
}
