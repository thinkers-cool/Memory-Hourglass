# Backend (src-tauri/)

Rust + Tauri 2 + SQLx SQLite. Specs: [docs/spec/](../docs/spec/README.md).

## Structure

| Path | Role |
|------|------|
| `src/lib.rs` | App setup, command registration, plugin init |
| `src/main.rs` | Binary entry |
| `src/state.rs` | `AppState`, `ActiveWorkspace` |
| `src/error.rs` | `AppError`, `ErrorCode`, `ErrorPayload` |
| `src/commands/` | Thin Tauri handlers (one file per domain) |
| `src/catalog/` | SQLite repos, models, rebuild, `pools.rs` |
| `src/scan/` | Discovery, indexing pipeline |
| `src/watcher/` | Filesystem notify + SMB poll |
| `src/library/` | Source roots, relink |
| `src/workspace/` | Workspace lifecycle, registry |
| `src/smb/` | Mount, keyring credentials (`macos.rs`, `linux.rs`, `unix.rs`, `windows.rs`) |
| `src/export/` | Copy/convert export |
| `src/query/` | Asset filtering, detail |
| `src/metadata/` | EXIF/XMP via exiftool-rs |
| `src/link/` | RAW↔JPEG pairing, duplicates |
| `src/thumb/` | Thumbnail generation |
| `src/jobs/` | Job mutex + cancel |
| `src/trace/` | Rotating log files, correlation IDs |
| `src/activity/` | Activity log, undo |
| `src/message/` | `message://notify` envelopes |
| `src/path_util.rs` | Unicode-safe path canonicalize, join, compare |
| `migrations/` | SQLx migrations |
| `tests/` | Integration tests |

## Commands

54 commands in `lib.rs`. Handlers in `src/commands/`:

| File | Domain |
|------|--------|
| `workspace.rs` | Create/open/close workspace |
| `library.rs` | Roots, relink, folder browse |
| `smb.rs` | Share listing, browse mount |
| `scan.rs` | Scan control, catalog rebuild |
| `asset.rs` | Meta update, soft delete, purge |
| `query.rs` | `query_assets`, `count_assets` |
| `tag.rs` | Tag CRUD, batch assign |
| `collection.rs` | Albums, smart collections |
| `export.rs` | Export start/cancel/status |
| `catalog.rs` | `rebuild_catalog` |
| `activity.rs` | `query_asset_activity`, `undo_activity` |

All return `crate::error::Result<T>`. Use `AppState::with_active` for workspace-scoped ops. IPC and events: [ARCHITECTURE.md](../docs/spec/ARCHITECTURE.md).

## State

```
AppState
├── workspaces: RwLock<WorkspaceService>
└── active: RwLock<Option<Arc<ActiveWorkspace>>>

ActiveWorkspace
├── catalog, library, collection, link
├── jobs: JobPool
├── scan_status, shutdown_tx, roots_refresh
```

One workspace open at a time. Close sends `shutdown_tx` to stop watcher/poller.

## Database

- Schema: `migrations/001_init.sql` (edit in place; no incremental migrations)
- Location: `{workspace}/catalog.db` (WAL, FK enabled)
- `CatalogPools`: read pool (5) + write pool (1)
- Tables, DTOs, enums: [DATA_MODEL.md](../docs/spec/DATA_MODEL.md)

## Services

| Service | Module | Role |
|---------|--------|------|
| `ScanService` | `scan/mod.rs` | WalkDir discovery, batch upsert, index queue |
| `WatcherService` | `watcher/mod.rs` | Local notify + SMB poll loop |
| `LibraryService` | `library/mod.rs` | Root CRUD, relink |
| `MetadataService` | `metadata/mod.rs` | EXIF + XMP sidecar |
| `ThumbService` | `thumb/mod.rs` | Thumbnail files in `thumbs/` |
| `LinkService` | `link/mod.rs` | RAW↔JPEG, content hash, duplicates |
| `ExportService` | `export/mod.rs` | Parallel copy/convert |
| `*Repo` | `catalog/repo.rs` | SQL queries |

## Testing

Integration tests: `src-tauri/tests/`. Unit tests in `#[cfg(test)]` modules. Policy and coverage: [CONVENTIONS.md](../docs/spec/CONVENTIONS.md#testing).

## Paths

- Use `path_util` for canonicalize, storage strings, joins, and comparisons.
- Store paths with `path_for_storage`; compare with `paths_equal`.
- Use `os_file_name`, `os_file_stem`, `os_extension` instead of `to_str()` on `OsStr`.
- Relative paths use `/` in the DB; `normalize_rel_path` accepts `\` on input.
- On Windows, `canonicalize` uses `\\?\` / `\\?\UNC\` for long and UNC paths.

## Patterns

- Thin handlers — logic in services/repos.
- Batch constants: `SCAN_BATCH_SIZE=250`, `INDEX_CHUNK_SIZE=8`.
- Cross-stack naming, security, testing: [CONVENTIONS.md](../docs/spec/CONVENTIONS.md).
- Workflows (scan, export, SMB): [WORKFLOWS.md](../docs/spec/WORKFLOWS.md).

## Do Not

- Add commands without registering in `lib.rs` invoke_handler.
- Skip `AppState::with_active` for workspace-scoped operations.
- Store credentials in SQLite or logs.
- Run export/rebuild while scan jobs are active (exclusive slot conflicts).
- Clear `source_root` during catalog rebuild.
