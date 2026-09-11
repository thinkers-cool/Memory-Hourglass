use crate::activity::record::record_catalog_rebuilt;
use crate::commands::context::begin_command;
use crate::error::Result;
use crate::state::AppState;
use tauri::State;

#[tauri::command] pub async fn rebuild_catalog(state: State<'_, AppState>) -> Result<()> {
    let correlation_id = begin_command("rebuild_catalog");
    state
        .with_active(|ws| async move {
            let pool = ws.catalog.pool().clone();
            let thumb_dir = ws.thumb_dir.clone();
            let roots_count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM source_root")
                .fetch_one(&pool)
                .await? as usize;
            crate::catalog::rebuild::rebuild_and_rescan(pool.clone(), thumb_dir).await?;
            record_catalog_rebuilt(&ws.activity, Some(&correlation_id), roots_count).await?;
            Ok(())
        })
        .await
}
