# TECHNICAL.md — Technical Specification

## Architecture

```
core/            gpm-core (Rust, edition 2024) — Tauri-free shared core
├── domain/      scan pipeline (finder → status checker → categorizer, rayon-parallel),
│                folder/settings/kanban/auth types
├── infrastructure/  git ops (git2 + git CLI), stores (JSON, atomic writes),
│                launcher (open in terminal/editor/URL), gh CLI, OAuth PKCE,
│                sync client, keyring token store
└── resources/   terminals.json / editors.json catalogs (compile-time embedded)

desktop/         Tauri 2 app — Windows/Linux (kanban + sync included)
├── src/         React 19 + TypeScript + Tailwind 4 (Vite)
└── src-tauri/   thin command shim over gpm-core (26 #[tauri::command] wrappers)

macos/           native macOS 26+ app (no kanban, no sync)
├── ffi/         gpm-ffi: UniFFI 0.32 staticlib over gpm-core
│                (proc-macro exports; async scan/pull/clean on tokio)
├── generated/   Swift bindings (build artifact, gitignored)
├── GitProjectsManager/  SwiftUI (Swift 6, @Observable, Liquid Glass)
├── project.yml  XcodeGen spec → GitProjectsManager.xcodeproj (gitignored)
└── scripts/build-rust.sh  cargo build + uniffi-bindgen (Xcode pre-build phase)

server/          axum + SQLite sync server (kanban state; Google OAuth)
```

## Shared persistence (`dirs::config_dir()/git-projects-manager/`)

Pretty JSON, camelCase, written atomically (temp file + rename). Both apps
read/write the same files: `config.json` (folders), `settings.json`,
`kanban_v2.json` + `repos_cache_v1.json` (Tauri only). Sync session in the OS
keychain (`keyring` with `apple-native`/`windows-native`/`sync-secret-service`
features; file fallback `session.json`, 0600).

## Sync configuration (build-time)

Kanban sync (Tauri only) reads three values from `core/src/config.rs`, each
resolved as compile-time env (`option_env!`) → dev runtime env → hardcoded
fallback: `GOOGLE_OAUTH_CLIENT_ID`, `GOOGLE_OAUTH_CLIENT_SECRET`,
`SYNC_SERVER_URL`. The **client secret fallback is empty in the public
source** — no credential ships in git; set `GOOGLE_OAUTH_CLIENT_SECRET` at
build time to enable sign-in. The client ID and server URL keep working
public-endpoint fallbacks. Desktop-client secrets are non-confidential to
Google, but are still kept out of source on principle.

## macOS build specifics

- UniFFI proc-macro setup (`uniffi::setup_scaffolding!`), library-mode bindgen
  against `libgpm_ffi.a`; the generated `gpm_ffi.swift` compiles into the app
  target; the FFI clang module is found via `SWIFT_INCLUDE_PATHS` +
  `module.modulemap`.
- Async exports use `#[uniffi::export(async_runtime = "tokio")]` and offload
  blocking work with `spawn_blocking` → Swift `async throws`.
- Link requirements beyond the staticlib: `-lz -liconv` (vendored libgit2),
  `Security.framework`, `SystemConfiguration.framework` (keyring/reqwest).
- `SWIFT_DEFAULT_ACTOR_ISOLATION = nonisolated` — UniFFI-generated code does
  not compile under a MainActor default (uniffi-rs #2818); app types opt into
  `@MainActor` explicitly.
- Not sandboxed (spawns `git`/`gh`/`osascript`/login shell; scans arbitrary
  paths). Ad-hoc codesigned for local builds.
- `xcodegen generate` requires the generated bindings to exist — run
  `macos/scripts/build-rust.sh` first (`just macos-project` does both).

## Scanning

- `Scanner` (core) walks each monitored folder (walkdir, ~60 excluded dir
  names, hidden dirs skipped), detects repos by `.git/`, checks status in
  parallel with rayon (git2 for branch/dirty; `git` CLI for
  upstream/fetch/ahead/behind), and detects uninitialized sibling directories.
- `onlyLocalChecks` per folder skips fetch + ahead/behind.
- Cancellation: `Arc<AtomicBool>` polled during directory walk only; a
  cancelled `Scanner` is replaced with a fresh instance. No UI currently
  exposes cancel.
- Unborn repos (no commits) detected via typed `git2::ErrorCode::UnbornBranch`.

## Quality gates

- `just clippy` — clippy pedantic, zero warnings across `core`,
  `desktop/src-tauri`, `macos/ffi` (CLAUDE.md requirement).
- `just test` — core unit tests (glob matcher etc.).
- Frontend: `pnpm build` (tsc strict + vite), eslint.

## Versions & releases

App version lives in `desktop/package.json` (tauri.conf.json reads it),
mirrored in `desktop/src-tauri/Cargo.toml` and `macos/project.yml`
(`MARKETING_VERSION`) — `scripts/deploy_releases.sh` bumps all three in one
commit; `core`/`ffi` crates track it manually.

`deploy_releases.sh` runs once per device and uploads that platform's
artifact to the shared GitHub release tag: **macOS → the native SwiftUI app**
(xcodebuild Release, host arch only, `MARKETING_VERSION` pinned to the release
version, zipped with `ditto` into `dist-release/`), Linux → Tauri `.deb`,
Windows → Tauri NSIS `.exe`. `scripts/install_release.sh` installs the latest
release: on macOS (26+ required) it stops any running instance — including an
older Tauri install, which it replaces at the same
`/Applications/Git Projects Manager.app` path — unzips, strips quarantine, and
registers with Launch Services. Bundles are ad-hoc signed
(signing/notarization: see ROADMAP.md).
