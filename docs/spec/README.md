# MemHG Specs

Architecture and domain reference for MemHG. Operational commands and agent boundaries: [AGENTS.md](../../AGENTS.md).

| Doc                                | Scope                                      |
| ---------------------------------- | ------------------------------------------ |
| [ARCHITECTURE.md](ARCHITECTURE.md) | System layers, app phases, IPC, module map |
| [DATA_MODEL.md](DATA_MODEL.md)     | SQLite schema, DTO fields, enums           |
| [WORKFLOWS.md](WORKFLOWS.md)       | Scan, index, watch, export, SMB, workspace |
| [CONVENTIONS.md](CONVENTIONS.md)   | Naming, testing, styling, i18n, security   |

**Granularity:** Each file covers one layer. `docs/spec/` = domain reference. `AGENTS.md` = commands and boundaries. `src/AGENTS.md` / `src-tauri/AGENTS.md` = layer structure and patterns. Use tables and file pointers; link instead of repeating.

**Source of truth:** Code wins when spec is stale. Update the matching spec when changing architecture, schema, or workflows (see [AGENTS.md](../../AGENTS.md#doc-maintenance)).
