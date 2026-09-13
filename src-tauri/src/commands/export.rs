use crate::activity::record::record_export_completed;
use crate::activity::ActivityRecorder;
use crate::catalog::models::ExportOptions;
use crate::commands::context::trace_command;
use crate::error::Result;
use crate::export::{ExportManifest, ExportService};
use crate::jobs::JobPool;
use crate::state::AppState;
use serde::Serialize;
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Runtime, State};

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum JobPhase {
    Started,
    Running,
    Completed,
    Failed,
}

#[derive(Clone, Serialize)]
pub struct JobProgressEvent {
    pub job_id: String,
    pub done: u64,
    pub total: u64,
    pub phase: JobPhase,
    pub message: String,
    pub file_name: Option<String>,
}

#[derive(Serialize)]
pub struct ExportStatus {
    pub job_id: Option<i64>,
    pub status: String,
    pub manifest: Option<ExportManifest>,
}

#[derive(Serialize)]
pub struct ExportJobSummary {
    pub id: i64,
    pub status: String,
    pub created_at: i64,
    pub copied_count: usize,
    pub failed_count: usize,
}

pub(crate) struct ExportJobParams<R: Runtime> {
    pub asset_ids: Vec<i64>,
    pub destination: String,
    pub opts: ExportOptions,
    pub job_id: i64,
    pub app: AppHandle<R>,
    pub export: ExportService,
    pub jobs: JobPool,
    pub cancel: Arc<AtomicBool>,
    pub activity_pools: crate::catalog::pools::CatalogPools,
}

pub(crate) async fn run_export_job<R: Runtime>(params: ExportJobParams<R>) {
    let ExportJobParams {
        asset_ids,
        destination,
        opts,
        job_id,
        app,
        export,
        jobs,
        cancel,
        activity_pools,
    } = params;
    let total = asset_ids.len() as u64;

    let _ = app.emit(
        "job://progress",
        JobProgressEvent {
            job_id: job_id.to_string(),
            done: 0,
            total,
            phase: JobPhase::Started,
            message: "export started".into(),
            file_name: None,
        },
    );
    tokio::task::yield_now().await;

    let progress_app = app.clone();
    let result = export
        .export_assets_with_progress(
            &asset_ids,
            Path::new(&destination),
            &opts,
            Some(cancel),
            Arc::new(move |processed, total, file_name, phase| {
                let _ = progress_app.emit(
                    "job://progress",
                    JobProgressEvent {
                        job_id: job_id.to_string(),
                        done: processed,
                        total,
                        phase: JobPhase::Running,
                        message: phase.into(),
                        file_name: file_name.map(str::to_string),
                    },
                );
            }),
        )
        .await;

    if let Ok(manifest) = &result {
        let recorder = ActivityRecorder::new(activity_pools.clone());
        let _ =
            record_export_completed(&recorder, None, job_id, asset_ids.len(), &destination).await;
        let (phase, message) = if manifest.failed.is_empty() {
            (JobPhase::Completed, "export completed".into())
        } else if manifest.copied.is_empty() {
            let detail = manifest
                .failed
                .first()
                .map(|f| f.error.as_str())
                .unwrap_or("unknown error");
            (JobPhase::Failed, format!("export failed: {}", detail))
        } else {
            (
                JobPhase::Completed,
                format!(
                    "export completed with errors ({} failed)",
                    manifest.failed.len()
                ),
            )
        };
        let _ = app.emit(
            "job://progress",
            JobProgressEvent {
                job_id: job_id.to_string(),
                done: total,
                total,
                phase,
                message,
                file_name: None,
            },
        );
    } else if let Err(error) = &result {
        let _ = app.emit(
            "job://progress",
            JobProgressEvent {
                job_id: job_id.to_string(),
                done: 0,
                total,
                phase: JobPhase::Failed,
                message: format!("export failed: {}", error),
                file_name: None,
            },
        );
    }

    jobs.finish_exclusive().await;
}

#[tauri::command]
pub async fn start_export<R: Runtime>(
    asset_ids: Vec<i64>,
    destination: String,
    options: Option<ExportOptions>,
    job_id: i64,
    app: AppHandle<R>,
    state: State<'_, AppState>,
) -> Result<i64> {
    trace_command("start_export", || async move {
        let (export, jobs, cancel, activity_pools) = state
            .with_active(|ws| async move {
                let cancel = ws.jobs.try_start_exclusive("export").await?;
                Ok((
                    ws.export_service(),
                    ws.jobs.clone(),
                    cancel,
                    ws.catalog.pools().clone(),
                ))
            })
            .await?;

        let opts = options.unwrap_or(ExportOptions {
            flat: true,
            rename_template: None,
            format: None,
        });
        tauri::async_runtime::spawn(run_export_job(ExportJobParams {
            asset_ids,
            destination,
            opts,
            job_id,
            app,
            export,
            jobs,
            cancel,
            activity_pools,
        }));

        Ok(job_id)
    })
    .await
}

#[tauri::command]
pub async fn cancel_export(state: State<'_, AppState>) -> Result<()> {
    trace_command("cancel_export", || async move {
        state
            .with_active(|ws| async move {
                ws.jobs.request_cancel_exclusive().await;
                Ok(())
            })
            .await
    })
    .await
}

#[tauri::command]
pub async fn get_export_status(state: State<'_, AppState>) -> Result<ExportStatus> {
    trace_command("get_export_status", || async move {
        state
            .with_active(|ws| async move {
                let latest = ws.export_service().latest_job().await?;
                Ok(match latest {
                    None => ExportStatus {
                        job_id: None,
                        status: "idle".into(),
                        manifest: None,
                    },
                    Some((id, status, manifest)) => ExportStatus {
                        job_id: Some(id),
                        status,
                        manifest: Some(manifest),
                    },
                })
            })
            .await
    })
    .await
}

#[tauri::command]
pub async fn list_export_jobs(state: State<'_, AppState>) -> Result<Vec<ExportJobSummary>> {
    trace_command("list_export_jobs", || async move {
        state
            .with_active(|ws| async move {
                let rows = ws.export_service().list_jobs().await?;
                Ok(rows
                    .into_iter()
                    .map(|(id, status, created_at, manifest)| ExportJobSummary {
                        id,
                        status,
                        created_at,
                        copied_count: manifest.copied.len(),
                        failed_count: manifest.failed.len(),
                    })
                    .collect())
            })
            .await
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{seed_scanned_asset, TauriFixture};

    #[tokio::test]
    async fn run_export_job_reports_total_failure() {
        let fixture = TauriFixture::new().await;
        let state = fixture.state();
        let handle = fixture.handle();
        let dest = fixture.hold.path().join("out");
        state
            .with_active(|ws| async move {
                let jobs = ws.jobs.clone();
                let cancel = jobs.try_start_exclusive("export").await.unwrap();
                run_export_job(ExportJobParams {
                    asset_ids: vec![999_999],
                    destination: dest.to_string_lossy().to_string(),
                    opts: ExportOptions {
                        flat: true,
                        rename_template: None,
                        format: None,
                    },
                    job_id: 1,
                    app: handle,
                    export: ws.export_service(),
                    jobs,
                    cancel,
                    activity_pools: ws.catalog.pools().clone(),
                })
                .await;
                Ok(())
            })
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn run_export_job_reports_destination_error() {
        let fixture = TauriFixture::new().await;
        let state = fixture.state();
        let handle = fixture.handle();
        let blocker = fixture.hold.path().join("blocker.txt");
        std::fs::write(&blocker, b"x").unwrap();
        state
            .with_active(|ws| async move {
                let jobs = ws.jobs.clone();
                let cancel = jobs.try_start_exclusive("export").await.unwrap();
                run_export_job(ExportJobParams {
                    asset_ids: vec![1],
                    destination: blocker.to_string_lossy().to_string(),
                    opts: ExportOptions {
                        flat: true,
                        rename_template: None,
                        format: None,
                    },
                    job_id: 2,
                    app: handle,
                    export: ws.export_service(),
                    jobs,
                    cancel,
                    activity_pools: ws.catalog.pools().clone(),
                })
                .await;
                Ok(())
            })
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn export_commands_cover_idle_cancel_and_listing() {
        let fixture = TauriFixture::new().await;
        let state = fixture.state();
        let handle = fixture.handle();

        let idle = get_export_status(state.clone()).await.unwrap();
        assert_eq!(idle.status, "idle");

        cancel_export(state.clone()).await.unwrap();

        let photos = fixture.hold.path().join("export-cmd");
        let (_root_id, asset_id) = seed_scanned_asset(&fixture, &photos, "exp.jpg").await;
        let dest = fixture.hold.path().join("export-dest");
        start_export(
            vec![asset_id],
            dest.to_string_lossy().to_string(),
            None,
            3,
            handle,
            state.clone(),
        )
        .await
        .unwrap();
        for _ in 0..100 {
            let status = get_export_status(state.clone()).await.unwrap();
            if status.status == "completed" || status.status == "failed" {
                assert!(status.job_id.is_some());
                assert!(status.manifest.is_some());
                let jobs = list_export_jobs(state.clone()).await.unwrap();
                assert!(!jobs.is_empty());
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
        panic!("export did not finish");
    }
}
