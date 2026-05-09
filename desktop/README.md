# Git Projects Manager

A desktop app for monitoring Git repositories across multiple folders.

## Features

- **Repository scanning** - Detect uncommitted changes and unpushed commits
- **Parallel scanning** - Fast multi-threaded scanning with Rust
- **Kanban board** - Track project priority with drag-and-drop
- **Cross-platform** - Windows, macOS, Linux

## Quick Start

```bash
pnpm install
pnpm run tauri:dev
```

## Tech Stack

- **Frontend**: React + TypeScript + Tailwind
- **Backend**: Tauri v2 (Rust)
- **Git**: git2 + git CLI

## Documentation

- [Quick Start](docs/QUICK_START.md)
- [Setup Guide](docs/SETUP.md)
- [Design & Architecture](docs/DESIGN.md)

## License

MIT
