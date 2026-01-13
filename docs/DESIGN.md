# Design

## Philosophy

Developers accumulate repositories across `~/projects`, `~/work`, `~/experiments`. Tracking which repos have uncommitted work or unpushed changes becomes a chore.

**Solution**: A single dashboard that watches multiple folders and shows repository status at a glance.

**Principles**:
1. Speed over features
2. Clarity over density
3. Native over web
4. Simple over configurable

## Architecture

**Frontend**: React + TypeScript + Tailwind CSS
**Backend**: Tauri v2 (Rust)
**Git**: git2 library + git CLI

### Why Tauri?
Electron bundles Chromium (~150MB). Tauri uses the system webview (~5MB). Rust gives us zero-cost abstractions, memory safety, and native filesystem performance.

### Project Structure

```
src/                           # React frontend
  ├── components/
  │   ├── folders/             # Folder management
  │   ├── scan/                # Scan results display
  │   └── kanban/              # Kanban board
  ├── hooks/                   # React hooks
  ├── lib/api.ts               # Tauri API wrapper
  └── types/index.ts           # TypeScript types

src-tauri/src/                 # Rust backend
  ├── commands/                # Tauri command handlers
  ├── domain/                  # Business logic
  │   ├── config.rs            # Configuration
  │   ├── scanner.rs           # Parallel scanner
  │   └── kanban.rs            # Kanban state
  └── infrastructure/          # Git operations, filesystem
```

## Repository Status

| Status | Meaning | Color |
|--------|---------|-------|
| Clean | No changes, nothing to push | Green |
| Uncommitted | Working tree has changes | Yellow |
| Unpushed | Local commits not on remote | Orange |
| Error | Couldn't read repository | Red |

## Kanban Board

Priority-based project tracking beyond git state.

| Column | Color |
|--------|-------|
| Backlog | Gray |
| Active - Low Prio. | Blue |
| Active - Mid Prio. | Yellow |
| Active - High Prio. | Red |
| Review | Yellow |
| Done | Green |

**Features**:
- Drag-and-drop between columns
- Notes on cards (Ctrl+Enter to save, Escape to cancel)
- Stale detection for removed repositories
- Orphaned cards display in Backlog

**Persistence**: `kanban.json` alongside `config.json`

### Tauri Drag-Drop Note

HTML5 drag-and-drop requires `"dragDropEnabled": false` in `tauri.conf.json`. Also, `dataTransfer.getData()` returns empty in Tauri builds, so we use React refs instead.

## Performance

- **Parallel scanning** with Rayon
- **Smart exclusions** (40+ patterns: node_modules, dist, .cache, etc.)
- **Cancelable** long-running operations
