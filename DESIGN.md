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
  dirs — though a folder that is itself a repo is never hidden by its name).
  Folders may not overlap: one inside another would scan shared repos twice at
  once, so the second is refused with the conflict named. Per-folder "only local
  checks" skips network round-trips, and the scan toolbar says so, since skipped
  checks would otherwise read as clean results.
- **Categorize repos**: Uncommitted Changes (yellow), Unpushed (orange),
  Unpulled (purple), Unpublished (blue — no remote, never pushed to a host),
  Remote Not Found (pink — a remote is configured but the host reports it is
  gone), Unknown Remote State (gray — the remote comparison was attempted and
  failed, so the repo is *not* silently reported as clean), Clean (green),
  Uninitialized (gray — directories with files but no git), Errors (red).
  Unpublished, Remote Not Found and Unknown Remote State are overlays: the repo
  also shows in its primary status section (e.g. Clean *and* Remote Not Found).
  Remote Not Found never fires on uncertainty (offline, auth, non-GitHub, no
  `gh`) — it needs `git fetch` to say "not found", `gh` to be signed in, and
  `gh` to confirm. The sign-in check matters because GitHub reports a private
  repo you cannot see as missing rather than forbidden, so an expired login
  would otherwise read as "every private repo was deleted".
- **Act on repos**: open in default terminal/editor/`lms-github`, reveal in
  Finder, copy path, fetch & pull (single or all unpulled), clean git-ignored
  files with exclude patterns (single or all clean).
- **Scan automatically**: on launch, on demand, and on window focus — the
  focus rescan shows the same progress as a manual scan (20s throttle).
- **Kanban board** (both apps): the user's GitHub repos as cards in five
  fixed columns (auto-populated via the `gh` CLI, drag to organize), with
  optional Google-sign-in cloud sync (see `server/`).

Authoritative behavior spec: [FRONTEND.md](./FRONTEND.md).

## Platform strategy

One Rust core, two frontends sharing the same layout: sidebar navigation, an
All Folders overview with every repo section expanded inline (Clean last),
per-folder detail, and the kanban board. Windows/Linux keep the Tauri app
(webview); macOS gets a full-parity native SwiftUI app (macOS 26 design
language, system light/dark) — macOS releases ship it, replacing any older
Tauri install. The kanban design language is defined by the SwiftUI app and
mirrored by the Tauri board. Both apps share on-disk state (including kanban
state and the sync session), so mixed setups stay consistent.
