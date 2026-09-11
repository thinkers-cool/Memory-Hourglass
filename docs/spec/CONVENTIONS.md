# Conventions

## Naming

| Layer | Convention | Example |
|-------|------------|---------|
| React components | PascalCase file + export | `AssetGridCard.tsx` |
| Hooks | `use` prefix, camelCase file | `useLibrary.ts` |
| lib utilities | camelCase file | `libraryFilters.ts` |
| Tauri commands | snake_case | `batch_append_tags` |
| TS API wrappers | camelCase → snake_case invoke | `createWorkspace` → `create_workspace` |
| DTO / JSON fields | snake_case | `file_name`, `capture_at` |
| TS locals / props | camelCase | `gridColumnCount` |
| Rust modules | lowercase | `index_pipeline` |
| Rust services | `*Service` | `ScanService` |
| Rust repos | `*Repo` | `AssetRepo` |
| DB columns / enum values | snake_case | `sync_state: "ok"` |
| Events | URI-style | `scan://progress` |
| localStorage keys | `memhg-*` prefix | `memhg-theme` |

## Frontend Patterns

- One component per file, named export.
- Pure logic in `src/lib/` — hooks wire state + API only.
- Mutations on `library.actions`, typed by `LibraryActions`.
- Dialogs: `*DialogOpen` boolean + `close*Dialog` callback.
- Co-located tests: `Foo.tsx` + `Foo.test.tsx`.
- Class helpers in `src/lib/*Class.ts` — compose Tailwind + DaisyUI utilities.

## Backend Patterns

- Thin command handlers in `src/commands/` — delegate to services.
- `AppState::with_active` for workspace-scoped commands.
- Errors: `crate::error::Result<T>` → `{ code, message }` JSON.
- Serde: `rename_all = "snake_case"` on enums exposed to frontend.
- Single job mutex via `JobQueue`.
- Batch sizes: scan upsert 250, index 32.

## Testing

### Frontend (Vitest)

- Config: `vitest.config.ts`; setup: `src/test/setup.ts`.
- Global Tauri mock; fixtures: `src/test/fixtures.ts`.
- Default: `npm test`. Full stack: `npm run test:all`.

**Coverage** (`npm run test:coverage`):

- Target: 100% lines, branches, functions, statements on runtime code under `src/`.
- Excluded: `*.test.*`, `src/test/**`, `src/main.tsx`, `src/types/**`, `src/**/types.ts`.

### Backend (cargo test)

- Integration: `src-tauri/tests/` (`e2e_workflow`, `library_features`, `collection_workflow`, `workspace_workflow`, `command_handlers/`).
- Unit tests in `#[cfg(test)]` modules; helpers: `AppState::test_with_fresh_workspace()`, `test_support.rs`.
- Default: `npm run test:rust` (`--test-threads=1`).

**Coverage** (`npm run test:rust:coverage`):

| Check | Threshold |
|-------|-----------|
| Zero-hit lines | 0 |
| Regions | 100% |

- In scope: service modules (`catalog/`, `scan/`, `query/`, `library/`, `export/`, `smb/`, …).
- Out of scope: `commands/**` (covered by `command_handlers/`), `main.rs`, `lib.rs`, `test_support.rs`, production `smb/credentials.rs`.
- Config: `src-tauri/.llvm-cov.toml`, `src-tauri/.cargo/config.toml`.
- Requires nightly Rust for gate runs.

Scan/SMB tests use `MEMHG_TEST_*` env vars; reset hooks between tests.

## Styling

- Tailwind 4 + DaisyUI 5 via `src/index.css`.
- 13 themes; default `slate`. Persisted `memhg-theme` on `<html data-theme>`.
- Semantic tokens in `src/styles/interaction.css`; per-theme overrides in `src/styles/themes/memhg.css`.
- Icons: `lucide-react`.
- Surface utilities: `.app-canvas`, `.surface-panel`, `.surface-toolbar`, `.surface-card`, `.surface-popover`, `.surface-float`.

| Module | Role |
|--------|------|
| `formControlClass.ts` | Input/select shells, menu lists |
| `interactionClass.ts` | Active/hover row and option states |
| `buttonClass.ts` | Ghost interactive buttons |
| `ratingStars.ts` | Star formatting |

## Internationalization

- Stack: `i18next` + `react-i18next`; bundled JSON in `src/locales/{en-US,zh-CN}/`.
- Init: `src/i18n/index.ts` (imported from `main.tsx` and test setup).
- Locale: `memhg-locale` storage, else `navigator.language` (`zh*` → `zh-CN`, else `en-US`).
- Namespaces: `common`, `library`, `dialogs`, `errors`.
- Components: `useTranslation`; `lib/` uses `i18n.t`.
- Do not translate: filenames, paths, tag/album names, user keywords. Tauri window title stays English.
- `LocaleSwitcher` above `ThemeSwitcher` in `AppearanceControls`.

## Security

- SMB passwords in OS keyring, never in DB or logs.
- `purge_delete` requires `"DELETE"` token; unavailable in read-only workspaces.
- No user input in shell commands or dynamic file paths without validation.
- Asset protocol scoped in `tauri.conf.json`.

## Error Payload

Backend errors serialize as `{ "code": "snake_case_code", "message": "human readable" }`. Codes in `src-tauri/src/error.rs` (`ErrorCode` enum). Frontend maps via `errors` namespace in `appError.ts`.
