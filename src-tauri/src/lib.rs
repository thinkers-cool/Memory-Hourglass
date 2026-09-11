#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

pub mod activity;
pub mod catalog;
pub mod collection;
pub mod dates;
pub mod error;
pub mod export;
pub mod jobs;
pub mod library;
pub mod link;
pub mod message;
pub mod metadata;
pub mod query;
pub mod scan;
pub mod sort;
pub mod smb;
pub mod thumb;
pub mod trace;
pub mod watcher;
pub mod workspace;

pub mod commands;
pub mod state;

#[cfg(test)]
mod test_support;

use tauri::Manager;
use tracing_appender::non_blocking::WorkerGuard;

struct TraceGuard {
    _guard: WorkerGuard,
}

pub fn bootstrap_app<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> crate::error::Result<()> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| crate::error::AppError::Library(error.to_string()))?;
    let log_dir = data_dir.join("logs");
    let guard = trace::init(&log_dir)?;
    app.manage(TraceGuard { _guard: guard });
    let state = state::AppState::new(data_dir)?;
    app.manage(state);
    Ok(())
}

pub fn configure_builder<R: tauri::Runtime>(
    builder: tauri::Builder<R>,
) -> tauri::Builder<R> {
    builder
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| bootstrap_app(app.handle()).map_err(std::convert::Into::into))
        .invoke_handler(tauri::generate_handler![
            commands::workspace::create_workspace,
            commands::workspace::open_workspace,
            commands::workspace::close_workspace,
            commands::workspace::get_active_workspace,
            commands::workspace::list_recent_workspaces,
            commands::workspace::remove_recent_workspace,
            commands::workspace::try_open_last_workspace,
            commands::library::add_root,
            commands::library::add_smb_source,
            commands::smb::list_smb_shares,
            commands::smb::mount_smb_for_browse,
            commands::library::list_folder_children,
            commands::library::relink_root,
            commands::library::list_root_stats,
            commands::library::preview_relink,
            commands::library::remove_root,
            commands::library::list_roots,
            commands::scan::start_scan,
            commands::scan::pause_scan,
            commands::scan::resume_scan,
            commands::scan::cancel_scan,
            commands::scan::get_scan_status,
            commands::asset::get_asset,
            commands::asset::update_asset_meta,
            commands::asset::batch_update_asset_meta,
            commands::asset::soft_delete_assets,
            commands::asset::purge_delete,
            commands::asset::restore_assets,
            commands::query::query_assets,
            commands::query::count_assets,
            commands::tag::list_tags,
            commands::tag::create_tag,
            commands::tag::update_tag,
            commands::tag::delete_tag,
            commands::tag::batch_append_tags,
            commands::tag::batch_remove_tags,
            commands::collection::list_smart_collections,
            commands::collection::save_smart_collection,
            commands::collection::delete_smart_collection,
            commands::collection::list_albums,
            commands::collection::create_album,
            commands::collection::update_album,
            commands::collection::delete_album,
            commands::collection::set_album_items,
            commands::collection::add_album_items,
            commands::collection::remove_album_items,
            commands::collection::get_album_asset_ids,
            commands::export::start_export,
            commands::export::cancel_export,
            commands::export::get_export_status,
            commands::export::list_export_jobs,
            commands::catalog::rebuild_catalog,
            commands::activity::query_asset_activity,
            commands::activity::undo_activity,
        ])
}

#[cfg(test)]
mod app_builder_tests {
    use tauri::Manager;

    #[test]
    fn configure_builder_builds_with_mock_runtime() {
        let app = super::configure_builder(tauri::test::mock_builder())
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .expect("build app");
        assert!(!app.handle().package_info().name.is_empty());
    }

    #[test]
    fn bootstrap_app_initializes_state() {
        let app = tauri::test::mock_builder()
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .expect("build app");
        super::bootstrap_app(app.handle()).expect("bootstrap");
        assert!(app.try_state::<super::state::AppState>().is_some());
    }
}
