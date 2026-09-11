<p align="center">
  <img src="MemHG.svg" width="256px" alt="Memory Hourglass"/>
</p>

# Memory Hourglass

Local-first photo and video library for desktop. Organize media from local folders and SMB shares; browse, tag, export.

## Features

- Multiple workspaces (separate libraries per project)
- Local folders and SMB network shares as sources
- Background scan and filesystem watch
- EXIF metadata, thumbnails, RAW/JPEG pairing, duplicate detection
- Tags, albums, ratings, smart collections
- Grid browse, inspector, full view, compare, slideshow
- Export with rename templates and format conversion

## Stack

| Layer | Technology |
|-------|------------|
| Desktop shell | [Tauri 2](https://v2.tauri.app/) |
| Frontend | React 19, TypeScript, Vite, Tailwind 4, DaisyUI 5 |
| Backend | Rust, Tokio, SQLx (SQLite) |
| Metadata | [exiftool-rs](https://github.com/Le-Syl21/exiftool-rs) |

## Prerequisites

- [Node.js](https://nodejs.org/) 20+
- [Rust](https://rustup.rs/) (stable)
- [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) for your platform

## Getting Started

```bash
npm install
npm run tauri dev
```

Frontend-only: `npm run dev`. Production build: `npm run tauri build`.

## Scripts

| Command | Description |
|---------|-------------|
| `npm run tauri dev` | Desktop app (development) |
| `npm run dev` | Vite dev server only |
| `npm run tauri build` | Production desktop build |
| `npm test` | Frontend tests |
| `npm run test:rust` | Rust tests |
| `npm run test:all` | Full test suite |

## Project Layout

```
src/              React UI, hooks, lib utilities
src-tauri/        Rust backend, Tauri commands, migrations
docs/spec/        Architecture and domain reference
AGENTS.md         Agent guide (commands, boundaries)
```

## Documentation

| Doc | Audience |
|-----|----------|
| [docs/spec/](docs/spec/README.md) | Architecture, data model, workflows, conventions |
| [AGENTS.md](AGENTS.md) | Build commands and doc maintenance |
| [src/AGENTS.md](src/AGENTS.md) | Frontend patterns |
| [src-tauri/AGENTS.md](src-tauri/AGENTS.md) | Backend patterns |

## License

[MIT](LICENSE) — Copyright (c) 2026 Shanghai Thinkers Technology Co., Ltd.
