# Workflows

Each section: behavior summary, then source files.

## Workspace

1. **Create:** User picks folder → `create_workspace` → writes `workspace.json`, opens DB, registers in `workspaces.json`.
2. **Open:** `open_workspace` → close prior → `ActiveWorkspace::open` → spawn watcher + SMB poller → `ensure_smb_mounts_ready`.
3. **Close:** `close_workspace` → `shutdown_tx` stops background tasks.
4. **Bootstrap:** `try_open_last_workspace` on app start.

Files: `src-tauri/src/workspace/`, `src/hooks/useWorkspace.ts`.

## Add Source Root

**Local:** `add_root(path)` → insert `source_root` → trigger scan.

**SMB:** `add_smb_source(SmbSourceInput)` — pre-mounted path or `Connect` with keyring credentials (`com.memhg.app`, key `smb:{host}:{share}:{user}`).

Mount dirs: `{app_data}/mounts/{ws-id}/_browse/` (browse), `_shares/` (library). Platforms: macOS (`mount_smbfs`), Linux (`mount -t cifs`), Windows (`net use`). Read-only workspaces mount SMB read-only on macOS (`-o ro`) and Linux (`ro`).

Files: `src-tauri/src/library/`, `src-tauri/src/smb/`, `src/components/layout/SmbConnectDialog.tsx`.

## Remove Source Root

`remove_root(id)` cancels scan for that root, drops it from the pending queue, clears scan status, waits for the job slot, deletes `source_root` (assets cascade). Notifies `roots_refresh`.

Files: `src-tauri/src/commands/library.rs`, `src-tauri/src/jobs/mod.rs`, `src-tauri/src/state.rs`.

## Scan Pipeline

Triggered by `start_scan(root_id)` or background watcher.

```
Discovery (WalkDir)
  → Inventory upsert (batch 250)
  → Mark missing paths
  → Index queue (chunks of 8)
  → Link pass (RAW↔JPEG, duplicates)
```

**Indexing:** One read per file → SHA-256, EXIF, embedded-thumb WebP (full decode fallback). One write transaction per chunk. `IoProfile::Network` caps index parallelism at 2 threads.

**Sync states:** `new`, `modified`, `ok`.

**Progress:** `scan://progress` (includes `root_id`); `scan://thumbs` per chunk. Frontend: per-root status in source panel (`scanStatus.ts` → `SourceScanProgress`); aggregate line in status bar (`statusBar.ts`).

**Concurrency:** `JobPool` — up to 2 parallel scans; additional roots queue. Export/rebuild use an exclusive slot. `resume_pending_scans` on library load for roots with incomplete indexing.

**Controls:** `cancel_scan(root_id?)`, `get_scan_status(root_id?)`, `list_scan_statuses`; `pause_scan`, `resume_scan` (backend only, no UI).

**Logging:** `{app_data}/logs/memhg.log` (default `memhg=debug`). Override with `RUST_LOG`.

Files: `src-tauri/src/scan/`, `src-tauri/src/commands/scan.rs`, `src/lib/scanStatus.ts`, `src/lib/statusBar.ts`.

## Filesystem Watch

**Local** (`scan_policy=watch`): `notify` recursive watcher, 3s debounce, `JobQueue::try_start("background-scan")`.

**SMB** (`kind=smb`): Poll loop; per-root interval from `poll_secs` (default 300s, min 30s).

**Roots refresh:** `roots_refresh` channel rebuilds watcher when roots change.

Files: `src-tauri/src/watcher/mod.rs`.

## Asset Query

`query_assets(filter, sort, offset, limit)` + `count_assets(filter)`. Filter: roots, tags, dates, rating, text, deleted, album/collection. Sort: `field:dir` in `sort.rs`.

Files: `src-tauri/src/query/`, `src/lib/libraryFilters.ts`, `src/hooks/library/useLibraryQuery.ts`.

## Metadata Edit

`update_asset_meta(id, patch)` or `batch_update_asset_meta(ids, patch)` patch rating only. Tags: `batch_append_tags` / `batch_remove_tags`.

Files: `src-tauri/src/commands/asset.rs`, `src-tauri/src/metadata/`, `src/hooks/library/createLibraryActions.ts`.

## Soft Delete / Purge

1. `soft_delete_assets(ids)` — sets `deleted_at`.
2. `restore_assets(ids)` — clears `deleted_at`.
3. `purge_delete(ids, confirm_token)` — requires `"DELETE"`, removes files and DB rows. Blocked in read-only workspaces.

Files: `src-tauri/src/commands/asset.rs`.

## Export

`start_export(asset_ids, destination, options?, job_id)`: parallel copy or convert (jpeg/webp/png). Options: `flat`, `rename_template`, `format`. Exclusive job slot. Progress: `job://progress`. Manifest in `export_job`.

Files: `src-tauri/src/export/`, `src/components/layout/ExportDialog.tsx`, `src/hooks/library/useLibraryExport.ts`.

## Catalog Rebuild

`rebuild_catalog` — clears catalog tables (not roots), rescans all roots. Records `catalog.rebuilt` in `activity_log`.

Files: `src-tauri/src/catalog/rebuild.rs`.

## Activity Log & Undo

User mutations append to `activity_log` with optional `revert_json`. `query_asset_activity`, `undo_activity` (Inspector). Scan/export/rebuild record aggregate events without undo.

Files: `src-tauri/src/activity/`, `src-tauri/src/commands/activity.rs`.

## Messages

Backend emits `message://notify` with i18n keys. Frontend: `useMessageSystem`, `message/bus.ts`, `StatusBar` / `StartStatusStrip`. Errors sticky; success auto-dismiss.

Files: `src-tauri/src/message/`, `src/hooks/useMessageSystem.ts`, `src/lib/statusBar.ts`.

## Relink Root

`preview_relink(id, new_path)` → `relink_root(id, new_path)`. Updates `source_root.path`; preserves assets by relative path.

Files: `src-tauri/src/library/`, `src-tauri/src/commands/library.rs`.

## Grid Interaction

Single click selects (deferred 400ms to allow double-click). Double click opens full view. Re-clicking sole selection deselects after the defer window.

Files: `src/lib/gridCardClick.ts`, `src/components/AssetGridCard.tsx`.
