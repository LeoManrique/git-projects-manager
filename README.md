# git-projects-manager

Monorepo with two projects:

- [`desktop/`](./desktop) — the Tauri desktop app (React + Rust).
- [`server/`](./server) — the optional sync server (axum + SQLite). Stores per-user kanban state, authenticated via Google OAuth.
- [`scripts/`](./scripts) — release/install scripts for the desktop app and the deploy script for the sync server.

The desktop app works standalone; the sync server is optional and only used when the user signs in.
