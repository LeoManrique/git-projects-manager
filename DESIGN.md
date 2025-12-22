# Design

This document outlines the design philosophy and architectural decisions for Git Projects Manager.

## 0. Philosophy

### The Problem

Developers accumulate repositories. Side projects, client work, open source contributions, experiments—they scatter across the filesystem like digital breadcrumbs. The mental overhead of tracking which repos have uncommitted work or unpushed changes grows with each new project.

Git's command-line tools are powerful but local. You can check *one* repository at a time. When you have dozens spread across `~/projects`, `~/work`, `~/experiments`, and that random folder on your Desktop, staying organized becomes a chore.

### The Solution

A single dashboard that watches multiple folders and tells you, at a glance:
- Which repositories need attention
- Which have uncommitted changes
- Which have commits waiting to be pushed
- Which are clean and synchronized

No configuration files to maintain. No terminal sessions to keep open. Just add your project folders and scan.

### Design Principles

1. **Speed over features** - Scanning should feel instant, even with hundreds of repositories
2. **Clarity over density** - Show what matters; hide what doesn't
3. **Native over web** - Desktop apps should feel like desktop apps
4. **Simple over configurable** - One workflow that works, not ten options to tune

## 1. Architecture

**Frontend**: React + TypeScript + Tailwind CSS
**Desktop Shell**: Tauri v2 (Rust)
**Git Operations**: git2 library + git CLI

### Why Tauri?

Electron bundles Chromium (~150MB). Tauri uses the system webview (~5MB). For an app that's essentially a dashboard with a file watcher, the lightweight approach wins. Rust also gives us:

- Zero-cost abstractions for parallel scanning
- Memory safety without garbage collection pauses
- Native performance for filesystem traversal

### Why Hybrid Git Operations?

The `git2` library handles most operations efficiently—detecting uncommitted changes, reading branch names, checking status. But some operations (like detecting unpushed commits with proper remote tracking) work more reliably through the git CLI. We use both:

- **git2**: Status checks, file detection, branch info
- **git CLI**: Remote comparisons, complex operations

## 2. Core Workflow

```
┌─────────────────────────────────────────────────────────┐
│                    User adds folder                      │
└─────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────┐
│              Folder saved to config.json                 │
│         (~/.config/.git-projects-manager/)               │
└─────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────┐
│                   User triggers scan                     │
└─────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────┐
│            Parallel traversal begins                     │
│    (Rayon threads, smart directory exclusions)           │
└─────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────┐
│          Each .git folder = one repository               │
│     Status checked: changes, unpushed, clean, error      │
└─────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────┐
│              Results grouped and displayed               │
│         (Expandable cards, color-coded status)           │
└─────────────────────────────────────────────────────────┘
```

## 3. Parallel Scanning

Repository scanning is embarrassingly parallel—each repo's status is independent. We use Rayon to check all repositories concurrently:

```
Folder with 100 repos:
  Sequential: ~10 seconds
  Parallel (8 cores): ~1.5 seconds
```

### Smart Exclusions

We skip 40+ directory patterns that never contain useful repositories:
- `node_modules`, `vendor`, `.venv` (dependencies)
- `build`, `dist`, `target` (outputs)
- `.cache`, `__pycache__` (caches)
- `.idea`, `.vscode` (IDE files)

This avoids traversing millions of files in typical JavaScript projects.

## 4. State Management

### Frontend (React)

Component-local state via hooks. No global state library—the app is simple enough that prop drilling works fine. The backend is the source of truth; the frontend just displays it.

**Transient** (resets on reload):
- Scan results
- Loading states
- Error messages
- Active tab

### Backend (Rust)

Thread-safe state with `Arc<RwLock<>>`:
- `ConfigManager` - Folder CRUD operations
- `Scanner` - Parallel scanning with cancellation support

**Persisted** (survives app restart):
- Monitored folders (path + display name)

Config location follows XDG conventions:
- macOS/Linux: `~/.config/.git-projects-manager/config.json`
- Windows: `%APPDATA%\.git-projects-manager\config.json`

## 5. Repository Status Model

Each repository falls into one of four states:

| Status | Meaning | Visual |
|--------|---------|--------|
| **Clean** | No changes, nothing to push | Green |
| **Uncommitted** | Working tree has changes | Yellow |
| **Unpushed** | Local commits not on remote | Orange |
| **Error** | Couldn't read repository | Red |

A repository can have both uncommitted changes AND unpushed commits—the UI shows both.

## 6. Project Structure

```
/src                          # React frontend
  ├── components/
  │   ├── FolderManager.tsx   # Add/edit/delete folders
  │   └── ScanResults.tsx     # Display scan results
  ├── lib/api.ts              # Type-safe Tauri bridge
  └── types/index.ts          # Shared interfaces

/src-tauri/src                # Rust backend
  ├── commands/               # Tauri command handlers
  │   ├── folder.rs           # Folder CRUD
  │   └── scan.rs             # Scanning operations
  ├── domain/                 # Business logic
  │   ├── config.rs           # Configuration persistence
  │   ├── scanner.rs          # Parallel scanner
  │   └── repository.rs       # Repo models
  └── infrastructure/         # External integrations
      ├── git.rs              # Git operations
      └── exclude.rs          # Directory exclusions
```

The structure follows a light domain-driven design:
- **Commands**: Thin layer exposing Tauri IPC
- **Domain**: Pure business logic, no external dependencies
- **Infrastructure**: Git library, filesystem, storage

## 7. Platform Support

### Current (v1.0)
- macOS (Apple Silicon + Intel)
- Windows (x64)
- Linux (x64, DEB/RPM/AppImage)

### Build Targets
- macOS: DMG, APP bundle
- Windows: MSI, NSIS installer
- Linux: DEB, RPM, AppImage

## 8. Future Considerations

- **Auto-scan on launch** - Optionally refresh status when app opens
- **File watching** - Detect changes in real-time without manual scanning
- **Quick actions** - Open terminal, open in editor, git pull from the UI
- **Repository filtering** - Search/filter by name, status, or path
- **Scan scheduling** - Background scans at configurable intervals
- **Notifications** - Alert when repositories need attention
