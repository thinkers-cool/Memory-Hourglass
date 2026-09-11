# Frontend (src/)

React 19 + TypeScript + Vite. Specs: [docs/spec/](../docs/spec/README.md).

## Structure

| Path | Contents |
|------|----------|
| `api/client.ts` | Sole Tauri bridge — all `invoke` and `listen` calls |
| `components/` | Feature components; subdirs: `layout/`, `shared/`, `slideshow/` |
| `hooks/` | React hooks; `hooks/library/` = `useLibrary` internals |
| `lib/` | Pure functions — filters, theme, selection, slideshow, message bus |
| `types/index.ts` | DTO types mirroring Rust (snake_case fields) |
| `styles/` | `interaction.css` tokens, `themes/memhg.css` overrides |
| `test/` | Vitest setup, fixtures |

## App Flow

```
main.tsx → App.tsx
  phase "loading"  → spinner
  phase "start"    → StartPage (workspace picker)
  phase "library"  → LibraryApp (main shell, lazy-loaded)
```

`LibraryApp` composes: `NavRail`, `LibraryPanel`, `VirtualGrid`, `InspectorPanel`, overlays (`GalleryPlayer`, `AssetFullView`, `CompareViewer`), dialogs, `StatusBar`, `FloatingSelectionBar`.

## State

| Hook | File | Role |
|------|------|------|
| `useWorkspace` | `hooks/useWorkspace.ts` | Phase, workspace, recent, notifications |
| `useLibrary` | `hooks/useLibrary.ts` | Facade over library sub-hooks |
| `useMessageSystem` | `hooks/useMessageSystem.ts` | Toasts, busy state, delayed undo IPC |
| `useLibraryQuery` | `hooks/library/useLibraryQuery.ts` | Grid data, filters, pagination, scan status |
| `useLibrarySelection` | `hooks/library/useLibrarySelection.ts` | Selected assets, compare mode |
| `createLibraryActions` | `hooks/library/createLibraryActions.ts` | All user mutations |

No global store. Components call `library.actions.*` for mutations.

## API Client

`api/client.ts` wraps Tauri IPC. Command names are snake_case (match Rust). Domain groups: workspace, roots/SMB, scan, assets, tags, albums/collections, export, activity. Events: `onScanProgress`, `onJobProgress`, `onMessageNotify`. Media: `convertFileSrc(path)` from `@tauri-apps/api/core`.

## Component Tiers

| Tier | Path | Examples |
|------|------|----------|
| Feature | `components/` | `VirtualGrid`, `AssetGridCard`, `GalleryPlayer` |
| Layout | `components/layout/` | `NavRail`, `LibraryPanel`, `InspectorPanel`, `*Dialog` |
| Shared | `components/shared/` | `FilterChipBar`, `SortDropdown`, `ThemeSwitcher` |
| Slideshow | `components/slideshow/` | `SlideStage`, `PhotoSlide`, `VideoSlide` |

## Key lib Modules

| Area | Modules |
|------|---------|
| Library | `libraryFilters.ts`, `libraryActions.ts`, `selection.ts`, `gridNavigation.ts` |
| UI classes | `formControlClass.ts`, `interactionClass.ts`, `buttonClass.ts` |
| App chrome | `theme.ts`, `locale.ts`, `i18n/`, `statusBar.ts`, `appError.ts` |
| Messages | `message/bus.ts` |
| Slideshow | `slideshow/*` |

Styling and i18n rules: [CONVENTIONS.md](../docs/spec/CONVENTIONS.md).

## Patterns

- Pure logic in `lib/`; thin components and hooks.
- UI strings via `useTranslation` (components) or `i18n.t` (`lib/`, action factories).
- `filterRef` in `useLibraryQuery` for stable async filter reads.
- Dialog state: `*DialogOpen` boolean + close callback.
- Virtualized grid via `@tanstack/react-virtual`.
- Read-only workspaces: purge UI and handlers disabled end-to-end.

## Testing

Co-located `*.test.ts(x)`. Setup: `src/test/setup.ts` (Tauri mock, happy-dom). Fixtures: `src/test/fixtures.ts`.

| Command | Use |
|---------|-----|
| `npm test` | Full frontend suite |
| `npm run test:coverage` | Coverage report |

Prefer targeted runs: `npx vitest run path/to/file.test.ts`. Coverage policy: [CONVENTIONS.md](../docs/spec/CONVENTIONS.md).

## Do Not

- Add API calls outside `api/client.ts`.
- Use snake_case for TS variable names (DTO fields only).
- Put business logic in components — use `lib/` or hooks.
- Add a client-side router — phases are intentional.
