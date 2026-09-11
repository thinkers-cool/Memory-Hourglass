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
| `src/catalog/` | SQLite repos, models, rebuild |
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
├── jobs: JobQueue
├── scan_status, shutdown_tx, roots_refresh
```

One workspace open at a time. Close sends `shutdown_tx` to stop watcher/poller.

## Database

- Schema: `migrations/001_init.sql` (edit in place; no incremental migrations)
- Applied via `sqlx::migrate!` in `catalog/mod.rs`
- Location: `{workspace}/catalog.db` (WAL, FK enabled)
- Detail: [DATA_MODEL.md](../docs/spec/DATA_MODEL.md)

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

| Command | Use |
|---------|-----|
| `npm run test:rust` | Full Rust suite (`--test-threads=1`) |
| `npm run test:rust:coverage` | Coverage gate (nightly, 100% regions) |

| File | Role |
|------|------|
| `tests/e2e_workflow.rs` | Scan → query → meta → tags → export |
| `tests/library_features.rs` | Albums, SMB, duplicates, relink |
| `tests/collection_workflow.rs` | Smart collection roundtrip |
| `tests/workspace_workflow.rs` | Workspace isolation |
| `tests/workspace_state_edges.rs` | Workspace/state error paths |
| `tests/command_handlers/` | Tauri command integration tests |

Unit tests in `#[cfg(test)]` modules. Helpers: `AppState::test_with_fresh_workspace()`, `test_support.rs`. Scan/SMB tests use `MEMHG_TEST_*` env vars — reset hooks between tests. Coverage policy: [CONVENTIONS.md](../docs/spec/CONVENTIONS.md).

## Patterns

- Thin handlers — logic in services/repos.
- `JobQueue::try_start(name)` — single concurrent background job.
- Batch constants: `SCAN_BATCH_SIZE=250`, `INDEX_BATCH_SIZE=32`.
- SMB creds via keyring (never in DB).
- `purge_delete` requires `confirm_token == "DELETE"`.
- Sort strings: `field:dir` parsed in `sort.rs`.

## Do Not

- Add commands without registering in `lib.rs` invoke_handler.
- Skip `AppState::with_active` for workspace-scoped operations.
- Store credentials in SQLite or logs.
- Run multiple scan/export jobs concurrently.
- Clear `source_root` during catalog rebuild.
