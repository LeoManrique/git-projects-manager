# git-projects-manager

Monorepo with two projects:

- [`desktop/`](./desktop) — the Tauri desktop app (React + Rust). Build/install scripts under `desktop/scripts/`.
- [`server/`](./server) — the optional sync server (axum + SQLite). Stores per-user kanban state, authenticated via Google OAuth.

The desktop app works standalone; the sync server is optional and only used when the user signs in.
