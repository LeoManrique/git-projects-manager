# DESIGN.md — Functional Design

## Problem

Developers accumulate git repositories across many folders (`~/Dev`, work
dirs, experiments). Uncommitted changes, unpushed commits, and never-pushed
projects silently pile up. The app is a single dashboard that watches chosen
folders and surfaces repository status at a glance.

## Principles

1. Speed over features — parallel native scanning, dense information display.
2. Clarity over density — fixed categories with stable colors.
3. Native over web — Tauri (system webview) on Windows/Linux; fully native
   SwiftUI on macOS.
4. Simple over configurable — few settings, sensible defaults.

## What it does

- **Monitor folders**: user registers folders; every scan discovers all git
  repos beneath them (smart exclusions: node_modules, target, caches, hidden
  dirs). Per-folder "only local checks" skips network round-trips.
- **Categorize repos**: Uncommitted Changes (yellow), Unpushed (orange),
  Unpulled (purple), Clean (green), Uninitialized (gray — directories with
  files but no git), Errors (red).
- **Act on repos**: open in default terminal/editor/`lms-github`, fetch & pull
  (single or all unpulled), clean git-ignored files with exclude patterns
  (single or all clean).
- **Scan automatically**: on launch, on demand, silently on window focus
  (20s throttle).
- **Kanban board** (Tauri app only): GitHub repos as cards in columns, with
  optional Google-sign-in cloud sync (see `server/`).

Authoritative behavior spec: [FRONTEND.md](./FRONTEND.md).

## Platform strategy

One Rust core, two frontends sharing the same layout: sidebar navigation, an
All Folders overview with actionable repos expanded inline, and per-folder
detail. Windows/Linux keep the Tauri app (webview, kanban included as a
sidebar view). macOS gets a native SwiftUI app (macOS 26 design language,
system light/dark) limited to folders/scan/settings. Both share on-disk
state, so they can be installed side by side.
