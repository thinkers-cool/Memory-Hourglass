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

## Scan Pipeline

Triggered by `start_scan(root_id)` or background watcher.

```
Discovery (WalkDir)
  → Inventory upsert (batch 250)
  → Mark missing paths
  → Index queue (batch 32; thumbs parallel capped, metadata via shared ExifTool)
  → Link pass (RAW↔JPEG, hashes, duplicates)
```

Sync states: `new`, `modified`, `ok`. Indexing: thumbnails first (parallel, capped), then metadata (shared ExifTool per batch) → `asset_meta`, `asset_raw_tag`, `thumb_key`. Progress: `scan://progress`; per-batch thumb paths: `scan://thumbs`. Frontend patches grid thumbs incrementally and debounces grid refresh during indexing. Controls: `cancel_scan`, `get_scan_status` (UI); `pause_scan`, `resume_scan` (backend IPC, no UI yet).

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
