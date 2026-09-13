# Workflows

## Workspace

1. **Create:** User picks folder → `create_workspace` → writes `workspace.json`, opens DB, registers in `workspaces.json`.
2. **Open:** `open_workspace` → close prior → `ActiveWorkspace::open` → spawn watcher + SMB poller → `ensure_smb_mounts_ready`.
3. **Close:** `close_workspace` → `shutdown_tx` stops background tasks.
4. **Bootstrap:** `try_open_last_workspace` on app start.

Files: `src-tauri/src/workspace/`, `src/hooks/useWorkspace.ts`.

## Add Source Root

**Local:** `add_root(path)` → insert `source_root` → trigger scan.

**SMB:** `add_smb_source(SmbSourceInput)` — pre-mounted path or `Connect` with keyring credentials (`com.memhg.app`, key `smb:{host}:{share}:{user}`).

Mount dirs: `{app_data}/mounts/{ws-id}/_browse/` (browse), `_shares/` (library). Platforms: macOS (`mount_smbfs`), Linux (`mount -t cifs`), Windows (`net use`). Read-only workspaces mount SMB shares read-only on macOS (`-o ro`) and Linux (`ro` mount option); read-write workspaces mount without that restriction.

Files: `src-tauri/src/library/`, `src-tauri/src/smb/`, `src/components/layout/SmbConnectDialog.tsx`.

## Remove Source Root

`remove_root(id)` cancels any running scan for that root, drops it from the pending scan queue, clears scan status, waits for the job slot to release, then deletes the `source_root` row (assets cascade). Notifies `roots_refresh` so the watcher rebuilds without the removed root.

Files: `src-tauri/src/commands/library.rs`, `src-tauri/src/jobs/mod.rs`, `src-tauri/src/state.rs`.

## Scan Pipeline

Triggered by `start_scan(root_id)` or background watcher.

```
Discovery (WalkDir)
  → Inventory upsert (batch 250)
  → Mark missing paths
  → Index queue (chunks of 8; pipelined index + DB commit; one file read → hash + EXIF + embedded-thumb WebP; ExifTool reused per thread)
  → Link pass (RAW↔JPEG, duplicates)
```

Sync states: `new`, `modified`, `ok`. Indexing: single read per file → SHA-256, one ExifTool pass (metadata + embedded `ThumbnailImage`/`PreviewImage`), WebP thumb from embedded JPEG when present (full decode fallback), → `asset_meta`, `asset_raw_tag`, `thumb_key`, `content_hash` (one write transaction per index chunk; next chunk indexes while prior chunk commits). `IoProfile::Network` (UNC, SMB, or mapped network drive) caps index parallelism at 2 threads. Progress: `scan://progress` (includes `root_id`); per-chunk thumb paths: `scan://thumbs`. Up to 2 scan jobs run in parallel (`JobPool`); additional roots queue until a slot frees. `resume_pending_scans` on library load rescans roots with `last_scan_at IS NULL` or incomplete indexing. Frontend maps progress per root in the source panel. Controls: `cancel_scan(root_id?)`, `get_scan_status(root_id?)`, `list_scan_statuses`; `pause_scan`, `resume_scan` (backend IPC, no UI yet). Export/rebuild use an exclusive job slot (blocks while scans run).

**Scan logging:** Rotating log at `{app_data}/logs/memhg.log` (default `memhg=debug`). Scan phases emit `info` lines: `scan job start/complete`, `scan discovery complete`, `scan catalog complete`, `scan index start/chunk/complete`, `index batch on disk`. Per-file breakdown (`read_ms`, `hash_ms`, `exif_ms`, `thumb_ms`, `used_exif_thumb`) at `debug`. Override with `RUST_LOG` (e.g. `memhg=info` to reduce noise).

**Catalog DB pools:** `CatalogPools` opens a read pool (5 connections, read-only) and a write pool (1 connection). Repos and services use the read pool for `SELECT` queries and the write pool for mutations. Activity logging and index-batch applies always go through the write pool.

Files: `src-tauri/src/scan/`, `src-tauri/src/commands/scan.rs`.

## Filesystem Watch

**Local** (`scan_policy=watch`): `notify` recursive watcher, 3s debounce, `JobQueue::try_start("background-scan")`.

**SMB** (`kind=smb`): Poll loop; per-root interval from `poll_secs` (default 300s, min 30s).

**Roots refresh:** `roots_refresh` channel rebuilds watcher when roots change.

Files: `src-tauri/src/watcher/mod.rs`.

## Asset Query

`query_assets(filter, sort, offset, limit)` + `count_assets(filter)`. Filter: roots, tags, dates, rating, text, deleted, album/collection. Sort: `field:dir` in `sort.rs`. Frontend: `libraryFilters.ts`, `useLibraryQuery.ts`.

Files: `src-tauri/src/query/`.

## Metadata Edit

`update_asset_meta(id, patch)` or `batch_update_asset_meta(ids, patch)` patch rating only. Tags use `batch_append_tags` / `batch_remove_tags`.

Files: `src-tauri/src/commands/asset.rs`, `src-tauri/src/metadata/`, `src/hooks/library/createLibraryActions.ts`.

## Soft Delete / Purge

1. `soft_delete_assets(ids)` — sets `deleted_at`.
2. `restore_assets(ids)` — clears `deleted_at`.
3. `purge_delete(ids, confirm_token)` — requires `"DELETE"`, removes files and DB rows. Blocked in read-only workspaces (UI hidden).

## Export

`start_export(asset_ids, destination, options?, job_id)`: parallel copy or convert (jpeg/webp/png). Options: `flat`, `rename_template`, `format`. Single job via `JobQueue`. Progress: `job://progress`. Manifest in `export_job`.

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
