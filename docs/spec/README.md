# MemHG Specs

Architecture and domain reference for MemHG. Operational commands and agent boundaries: [AGENTS.md](../../AGENTS.md).

| Doc                                | Scope                                      |
| ---------------------------------- | ------------------------------------------ |
| [ARCHITECTURE.md](ARCHITECTURE.md) | System layers, app phases, IPC, module map |
| [DATA_MODEL.md](DATA_MODEL.md)     | SQLite schema, DTO fields, enums           |
| [WORKFLOWS.md](WORKFLOWS.md)       | Scan, index, watch, export, SMB, workspace |
| [CONVENTIONS.md](CONVENTIONS.md)   | Naming, testing, styling, i18n, security   |

Keep specs dry and factual. Use tables and pointers to source files; avoid duplicating stack guides in [src/AGENTS.md](../../src/AGENTS.md) and [src-tauri/AGENTS.md](../../src-tauri/AGENTS.md).

**Source of truth:** Code wins when spec is stale. Update the relevant spec file when changing architecture, schema, or workflows.
