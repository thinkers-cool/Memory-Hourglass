use crate::catalog::models::{Album, SmartCollection};
use crate::commands::context::trace_command;
use crate::error::Result;
use crate::query::AssetFilter;
use crate::sort::default_album_sort;
use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub async fn list_smart_collections(state: State<'_, AppState>) -> Result<Vec<SmartCollection>> {
    trace_command("list_smart_collections", |_correlation_id| async move {
        state
            .with_active(|ws| async move {
                let collections = ws.collection.list_smart_collections().await?;
                let mut enriched = Vec::with_capacity(collections.len());
                for collection in collections {
                    let asset_count = ws.query_service().count(&collection.filter).await?;
                    enriched.push(SmartCollection {
                        id: collection.id,
                        name: collection.name,
                        filter: collection.filter,
                        asset_count,
                    });
                }
                Ok(enriched)
            })
            .await
    })
    .await
}

#[tauri::command]
pub async fn save_smart_collection(
    name: String,
    filter: AssetFilter,
    state: State<'_, AppState>,
) -> Result<SmartCollection> {
    trace_command("save_smart_collection", |_correlation_id| async move {
        state
            .with_active(|ws| async move {
                let collection = ws.collection.save_smart_collection(&name, &filter).await?;
                let asset_count = ws.query_service().count(&collection.filter).await?;
                Ok(SmartCollection {
                    id: collection.id,
                    name: collection.name,
                    filter: collection.filter,
                    asset_count,
                })
            })
            .await
    })
    .await
}

#[tauri::command]
pub async fn delete_smart_collection(id: i64, state: State<'_, AppState>) -> Result<()> {
    trace_command("delete_smart_collection", |_correlation_id| async move {
        state
            .with_active(|ws| async move { ws.collection.delete_smart_collection(id).await })
            .await
    })
    .await
}

#[tauri::command]
pub async fn list_albums(state: State<'_, AppState>) -> Result<Vec<Album>> {
    trace_command("list_albums", |_correlation_id| async move {
        state
            .with_active(|ws| async move { ws.collection.list_albums().await })
            .await
    })
    .await
}

#[tauri::command]
pub async fn create_album(
    name: String,
    sort_mode: Option<String>,
    emoji: Option<String>,
    state: State<'_, AppState>,
) -> Result<Album> {
    trace_command("create_album", |_correlation_id| async move {
        state
            .with_active(|ws| async move {
                let sort = sort_mode.unwrap_or_else(|| default_album_sort().encode());
                ws.collection
                    .create_album(&name, &sort, emoji.as_deref())
                    .await
            })
            .await
    })
    .await
}

#[tauri::command]
pub async fn update_album(
    id: i64,
    name: String,
    emoji: Option<String>,
    state: State<'_, AppState>,
) -> Result<Album> {
    trace_command("update_album", |_correlation_id| async move {
        state
            .with_active(|ws| async move {
                ws.collection
                    .update_album(id, &name, emoji.as_deref())
                    .await
            })
            .await
    })
    .await
}

#[tauri::command]
pub async fn delete_album(id: i64, state: State<'_, AppState>) -> Result<()> {
    trace_command("delete_album", |_correlation_id| async move {
        state
            .with_active(|ws| async move { ws.collection.delete_album(id).await })
            .await
    })
    .await
}

#[tauri::command]
pub async fn set_album_items(
    album_id: i64,
    asset_ids: Vec<i64>,
    state: State<'_, AppState>,
) -> Result<()> {
    trace_command("set_album_items", |_correlation_id| async move {
        state
            .with_active(
                |ws| async move { ws.collection.set_album_items(album_id, &asset_ids).await },
            )
            .await
    })
    .await
}

#[tauri::command]
pub async fn add_album_items(
    album_id: i64,
    asset_ids: Vec<i64>,
    state: State<'_, AppState>,
) -> Result<u64> {
    trace_command("add_album_items", |_correlation_id| async move {
        state
            .with_active(
                |ws| async move { ws.collection.add_album_items(album_id, &asset_ids).await },
            )
            .await
    })
    .await
}

#[tauri::command]
pub async fn remove_album_items(
    album_id: i64,
    asset_ids: Vec<i64>,
    state: State<'_, AppState>,
) -> Result<u64> {
    trace_command("remove_album_items", |_correlation_id| async move {
        state
            .with_active(|ws| async move {
                ws.collection.remove_album_items(album_id, &asset_ids).await
            })
            .await
    })
    .await
}

#[tauri::command]
pub async fn get_album_asset_ids(album_id: i64, state: State<'_, AppState>) -> Result<Vec<i64>> {
    trace_command("get_album_asset_ids", |_correlation_id| async move {
        state
            .with_active(|ws| async move { ws.collection.album_asset_ids(album_id).await })
            .await
    })
    .await
}
