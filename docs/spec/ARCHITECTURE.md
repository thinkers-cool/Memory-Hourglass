# Architecture

## Stack

| Layer | Technology |
|-------|------------|
| Shell | Tauri 2 (`com.memhg.app`) |
| Frontend | React 19, TypeScript, Vite 6, Tailwind 4, DaisyUI 5 |
| Backend | Rust 2021, Tokio, SQLx (SQLite) |
| Media | exiftool-rs, image crate, asset protocol |

## Layers

```
┌─────────────────────────────────────────┐
│  Components (src/components/)           │
│  Hooks (src/hooks/)                     │
├─────────────────────────────────────────┤
│  lib/ — pure logic, no React            │
├─────────────────────────────────────────┤
│  api/client.ts — invoke + listen        │
├─────────────────────────────────────────┤
│  commands/ — Tauri handlers (thin)      │
├─────────────────────────────────────────┤
│  Services — scan, catalog, library, …   │
├─────────────────────────────────────────┤
│  catalog.db (SQLite, per workspace)     │
└─────────────────────────────────────────┘
```

## App Phases

No client-side router. `useWorkspace` drives phase:

| Phase | UI | Trigger |
|-------|-----|---------|
| `loading` | Spinner | Bootstrap |
| `start` | `StartPage` | No workspace open |
| `library` | `LibraryApp` | Workspace open |

Bootstrap calls `try_open_last_workspace`. On success, skips `StartPage`.

## Frontend State

No global store. `useWorkspace` (phase, workspace), `useLibrary` (facade), `createLibraryActions` (mutations). Detail: [src/AGENTS.md](../../src/AGENTS.md).

## Backend State

`AppState` (`src-tauri/src/state.rs`):

- `workspaces` — app-data registry
- `active: Option<Arc<ActiveWorkspace>>` — one open workspace at a time

`ActiveWorkspace` holds catalog, library, jobs, scan status, watcher shutdown channel. Commands use `AppState::with_active`. Detail: [src-tauri/AGENTS.md](../../src-tauri/AGENTS.md).

## IPC

**Commands:** `invoke(command_name, args)` — snake_case names in `src-tauri/src/lib.rs`.

**Events:**

| Channel | Payload | Source |
|---------|---------|--------|
| `scan://progress` | `ScanProgress` | Scan pipeline |
| `job://progress` | `JobProgress` | Export jobs |
| `message://notify` | `MessageEnvelope` | User-facing toasts |

**Observability:**

| System | Storage | Role |
|--------|---------|------|
| Trace | `{app_data}/logs/memhg.log` | Developer diagnostics |
| Activity | `activity_log` table | Asset history + undo |
| Message | Frontend bus + Tauri events | User toasts |

All three share `correlation_id` per command.

**Media URLs:** `convertFileSrc(path)` for thumbnails and full-size assets.

## Module Map (Backend)

| Module | Path | Role |
|--------|------|------|
| `catalog` | `src/catalog/` | SQLite repos, rebuild |
| `collection` | `src/collection/` | Albums, smart collections |
| `library` | `src/library/` | Source roots, relink |
| `scan` | `src/scan/` | Discovery, indexing |
| `watcher` | `src/watcher/` | Local notify + SMB poll |
| `workspace` | `src/workspace/` | Lifecycle, registry |
| `smb` | `src/smb/` | Mount, keyring creds |
| `export` | `src/export/` | Copy/convert export |
| `query` | `src/query/` | Asset filter, detail |
| `metadata` | `src/metadata/` | EXIF/XMP |
| `link` | `src/link/` | RAW↔JPEG, duplicates |
| `thumb` | `src/thumb/` | Thumbnail generation |
| `jobs` | `src/jobs/` | Single-job mutex |

## Module Map (Frontend)

| Path | Role |
|------|------|
| `src/components/layout/` | Shell: NavRail, LibraryPanel, InspectorPanel, dialogs |
| `src/components/shared/` | Reusable controls, filters, pickers |
| `src/components/slideshow/` | Gallery playback |
| `src/hooks/library/` | Decomposed `useLibrary` internals |
| `src/lib/` | Pure utilities, filters, theme, slideshow engine |

## On-Disk Layout

```
{workspace}/
  workspace.json
  catalog.db
  thumbs/

{app_data}/
  workspaces.json
  mounts/{ws-id}/
    _browse/
    _shares/
```

Workspace IDs: `ws-{timestamp_millis}`.
