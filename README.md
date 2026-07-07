# git-projects-manager

Monitor the git status of every repository under your project folders — uncommitted
changes, unpushed/unpulled commits, never-published (no-remote) projects, and forgotten
uninitialized directories — from one dashboard.

Monorepo layout:

- [`core/`](./core) — `gpm-core`, the shared Rust core: repository scanning, git
  operations, settings and persistence. Both frontends sit on it and share the same
  on-disk stores.
- [`desktop/`](./desktop) — the Tauri app (React + Rust) for **Windows and Linux**.
- [`macos/`](./macos) — the **native SwiftUI app for macOS 26+**, bridged to
  `gpm-core` via UniFFI. Full feature parity with the Tauri app.

Both apps include the GitHub-backed kanban board with optional cloud sync.
- [`server/`](./server) — the optional sync server (axum + SQLite). Stores per-user
  kanban state, authenticated via Google OAuth.
- [`scripts/`](./scripts) — release/install scripts for the apps (macOS releases ship
  the native SwiftUI app; Linux/Windows the Tauri app) and the deploy script for the
  sync server.

Frontend behavior for both apps is specified in [FRONTEND.md](./FRONTEND.md) — the
single source of truth. Architecture details live in [TECHNICAL.md](./TECHNICAL.md).

The apps work standalone; the sync server is optional and only used when the user
signs in. Sign-in needs your own Google OAuth "Desktop app" credentials — the
public source ships no client secret; supply one at build time via
`GOOGLE_OAUTH_CLIENT_SECRET` (see [TECHNICAL.md](./TECHNICAL.md)).

## License

MIT — see [LICENSE](./LICENSE).

## Quick start

```sh
just install       # desktop + server deps
just dev           # Tauri app (dev)
just dev-macos     # native macOS app (Debug build + launch)
just clippy        # pedantic lint across all Rust crates
just test          # Rust tests
```
