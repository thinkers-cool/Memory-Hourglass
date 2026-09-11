# MemHG — Agent Guide

Local-first photo library desktop app. **Memory Hourglass** (`com.memhg.app`). Tauri 2 + React 19 + Rust + SQLite.

## Commands

| Task                | Command                      |
| ------------------- | ---------------------------- |
| Dev (full app)      | `npm run tauri dev`          |
| Frontend only       | `npm run dev`                |
| Build               | `npm run tauri build`        |
| Frontend tests      | `npm test`                   |
| Frontend coverage   | `npm run test:coverage`      |
| Rust tests          | `npm run test:rust`          |
| Rust test coverage  | `npm run test:rust:coverage` |
| All tests           | `npm run test:all`           |
| Frontend lint       | `npm run lint`               |
| Format check (TS)   | `npm run fmt`                |
| Format check (Rust) | `npm run fmt:rust`           |

Run relevant tests before finishing work. Prefer `npm run test:all` for cross-stack changes.

## Repository Map

| Path         | Role                          | Detail                                     |
| ------------ | ----------------------------- | ------------------------------------------ |
| `src/`       | React frontend                | [src/AGENTS.md](src/AGENTS.md)             |
| `src-tauri/` | Rust backend + Tauri commands | [src-tauri/AGENTS.md](src-tauri/AGENTS.md) |
| `docs/spec/` | Architecture and domain specs | [docs/spec/README.md](docs/spec/README.md) |

## Architecture (summary)

```
React UI (src/)
  └─ api/client.ts  →  Tauri invoke/listen
       └─ commands/ (src-tauri/src/commands/)
            └─ services: catalog, scan, library, export, smb, workspace
                 └─ SQLite catalog.db per workspace
```

Phase-based routing (no React Router): `StartPage` → `LibraryApp`. See [docs/spec/ARCHITECTURE.md](docs/spec/ARCHITECTURE.md).

## Boundaries

**Always**

- Match existing naming: snake_case DTO fields, camelCase TS locals, snake_case Rust/Tauri commands.
- Keep business logic in `src/lib/` (frontend) or service modules (backend), not in components/command handlers.
- Add co-located `*.test.ts(x)` for new frontend logic; Rust unit/integration tests for backend behavior.
- Update spec docs when changing schema, commands, workflows, or architecture (see Doc Maintenance).

**Ask first**

- New npm or Cargo dependencies.
- Database schema changes (edit `001_init.sql` in place).
- New Tauri commands or event channels.
- Breaking changes to `src/types/index.ts` DTOs.

**Never**

- Commit secrets, credentials, or `.env` files.
- Use `eval`, dynamic `require`, or shell execution with user-derived paths.
- Hard-reset git history or skip hooks unless explicitly requested.
- Purge-delete assets without the `confirm_token` `"DELETE"` guard on the backend.

## Doc Maintenance

Specs live in `docs/spec/`. When you change code in these areas, update the matching doc in the same PR:

| Code change                                  | Update                      |
| -------------------------------------------- | --------------------------- |
| Module layout, app phases, IPC pattern       | `docs/spec/ARCHITECTURE.md` |
| i18n locales, namespaces, translation rules  | `docs/spec/CONVENTIONS.md`  |
| SQL schema, migrations, `src/types/index.ts` | `docs/spec/DATA_MODEL.md`   |
| Scan, watch, export, SMB, workspace flows    | `docs/spec/WORKFLOWS.md`    |
| Naming, testing, styling rules               | `docs/spec/CONVENTIONS.md`  |
| Frontend-only patterns                       | `src/AGENTS.md`             |
| Backend-only patterns                        | `src-tauri/AGENTS.md`       |

Keep docs dry and factual at a consistent granularity: tables and pointers in AGENTS.md and docs/spec/; avoid repeating deep dives across files.

## Key Entry Points

| File                                | Purpose                         |
| ----------------------------------- | ------------------------------- |
| `src/main.tsx`                      | Frontend bootstrap              |
| `src/App.tsx`                       | Workspace phase routing         |
| `src/api/client.ts`                 | All Tauri IPC                   |
| `src/types/index.ts`                | Shared DTO types                |
| `src-tauri/src/lib.rs`              | Command registration, app setup |
| `src-tauri/src/state.rs`            | `AppState`, `ActiveWorkspace`   |
| `src-tauri/migrations/001_init.sql` | Database schema                 |
