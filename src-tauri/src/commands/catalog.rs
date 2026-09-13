use crate::activity::record::record_catalog_rebuilt;
use crate::activity::ActivityRecorder;
use crate::commands::context::trace_command_with_id;
use crate::error::Result;
use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub async fn rebuild_catalog(state: State<'_, AppState>) -> Result<()> {
    trace_command_with_id("rebuild_catalog", |correlation_id| async move {
        let (pools, thumb_dir, jobs, roots_count, activity_pools) = state
            .with_active(|ws| async move {
                let _cancel = ws.jobs.try_start_exclusive("rebuild").await?;
                let roots_count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM source_root")
                    .fetch_one(ws.catalog.pool())
                    .await? as usize;
                Ok((
                    ws.catalog.pools().clone(),
                    ws.thumb_dir.clone(),
                    ws.jobs.clone(),
                    roots_count,
                    ws.catalog.pools().clone(),
                ))
            })
            .await?;

        tauri::async_runtime::spawn(async move {
            let result: Result<()> = async {
                crate::catalog::rebuild::rebuild_and_rescan(pools, thumb_dir).await?;
                let activity = ActivityRecorder::new(activity_pools);
                record_catalog_rebuilt(&activity, Some(&correlation_id), roots_count).await?;
                Ok(())
            }
            .await;
            if let Err(error) = result {
                tracing::error!(correlation_id = %correlation_id, error = %error, "rebuild_catalog failed");
            }
            jobs.finish_exclusive().await;
        });

        Ok(())
    })
    .await
}
