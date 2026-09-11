# Data Model

Schema source: `src-tauri/migrations/001_init.sql`. TypeScript mirror: `src/types/index.ts`. Field names use **snake_case** in JSON and DTOs.

## Tables

### source_root

Library source (local folder or SMB share).

| Column | Type | Notes |
|--------|------|-------|
| `path` | TEXT UNIQUE | Absolute path or mount point |
| `kind` | TEXT | `local`, `smb` |
| `scan_policy` | TEXT | `watch`, `poll`, `manual` |
| `poll_secs` | INTEGER | SMB poll interval (min 30) |
| `status` | TEXT | `idle`, `scanning`, `offline` |
| `smb_host`, `smb_share`, `smb_username` | TEXT | SMB metadata |
| `smb_mounted` | INTEGER | 0/1 |

### asset

Indexed file per root.

| Column | Type | Notes |
|--------|------|-------|
| `root_id` | FK | → source_root |
| `rel_path` | TEXT | Relative to root |
| `file_name`, `ext` | TEXT | |
| `kind` | TEXT | `image`, `raw`, `video` |
| `sync_state` | TEXT | `new`, `modified`, `ok` |
| `content_hash` | TEXT | SHA-256 for duplicate detection |
| `thumb_key` | TEXT | Thumbnail filename |
| `deleted_at` | INTEGER | Soft delete timestamp |
| `has_duplicate` | INTEGER | 0/1 flag |
| `indexed_mtime_ns` | INTEGER | Mtime at last index |

Unique: `(root_id, rel_path)`.

### asset_meta

EXIF-derived metadata. 1:1 with asset.

| Column | Notes |
|--------|-------|
| `capture_at` | Unix timestamp |
| `camera`, `lens` | TEXT |
| `rating` | INTEGER 0–5 |
| `latitude`, `longitude` | REAL |
| `keywords_json` | JSON array string |

### tag / asset_tag

Hierarchical tags. `tag.parent_id` self-references. `asset_tag` is many-to-many.

### asset_raw_tag

Raw EXIF name/value pairs per asset.

### asset_link

Relationships between assets.

| `kind` values | Meaning |
|---------------|---------|
| `raw_jpeg` | RAW paired with JPEG |
| `duplicate` | Content-hash duplicate |

### album / album_item

Manual albums. `album_item` has `position` for ordering.

### smart_collection

Named saved filter. `filter_json` serializes `AssetFilter`.

### export_job

Export history. `manifest_json` holds file list and status.

### activity_log

Append-only workspace activity history.

| Column | Notes |
|--------|-------|
| `seq` | Monotonic workspace sequence |
| `occurred_at` | Unix timestamp |
| `event_type` | e.g. `asset.metadata_changed`, `scan.completed` |
| `actor` | `user`, `scan`, `watcher`, `export`, `system` |
| `correlation_id` | Links to trace logs and messages |
| `subject_type` / `subject_id` / `subject_key` | Asset identity (`subject_key` = `{root_id}:{rel_path}`) |
| `payload_json` | Event facts (after-state, counts) |
| `revert_json` | Undo data when reversible |
| `undone_at` / `undone_by_id` | Undo chain |

Not cleared by `rebuild_catalog`.

## Key DTOs (TypeScript)

Defined in `src/types/index.ts`:

| Type | Use |
|------|-----|
| `WorkspaceInfo`, `RecentWorkspace` | Workspace lifecycle |
| `SourceRoot`, `RootStats` | Library roots |
| `AssetCard` | Grid display |
| `Asset`, `AssetDetail` | Full asset + meta + tags |
| `AssetFilter` | Query filter (roots, tags, dates, rating, text) |
| `TagDto`, `Album`, `SmartCollection` | Catalog entities |
| `ScanProgress`, `JobProgress` | Event payloads |
| `ActivityEntry` | Asset/workspace activity history |
| `ExportOptions`, `ExportManifest` | Export |

## Enums

| Domain | Values |
|--------|--------|
| `kind` (root) | `local`, `smb` |
| `scan_policy` | `watch`, `poll`, `manual` |
| `kind` (asset) | `image`, `raw`, `video` |
| `sync_state` | `new`, `modified`, `ok` |
| `SortMode` | `date`, `name`, `rating`, `path` |
| `SortDir` | `asc`, `desc` |
| Sort string format | `field:dir` e.g. `date:desc` |

## Catalog Rebuild

`rebuild_catalog` clears: `album_item`, `album`, `smart_collection`, `asset_link`, `asset_tag`, `asset_raw_tag`, `asset_meta`, `asset`, `tag`, `export_job`. Does **not** clear `source_root`.
