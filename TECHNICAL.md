# TECHNICAL.md — Technical Specification

## Architecture

```
core/            gpm-core (Rust, edition 2024) — Tauri-free shared core
├── domain/      scan pipeline (finder → status checker → categorizer, rayon-parallel),
│                folder/settings/kanban/auth types
├── infrastructure/  git ops (git2 + git CLI), stores (JSON, atomic writes),
│                launcher (open in terminal/editor/URL), gh CLI, OAuth PKCE,
│                sync client, keyring token store
├── services/    shared orchestration: kanban refresh/move/delete + cloud
│                sync merge, Google sign-in/out (used by both frontends)
└── resources/   terminals.json / editors.json catalogs (compile-time embedded)

desktop/         Tauri 2 app — Windows/Linux
├── src/         React 19 + TypeScript + Tailwind 4 (Vite)
└── src-tauri/   thin command shim over gpm-core (#[tauri::command] wrappers)

macos/           native macOS 26+ app — full parity (kanban + sync included)
├── ffi/         gpm-ffi: UniFFI 0.32 staticlib over gpm-core
│                (proc-macro exports; async scan/git/kanban/sync on tokio)
├── generated/   Swift bindings (build artifact, gitignored)
├── GitProjectsManager/  SwiftUI (Swift 6, @Observable, Liquid Glass)
│   └── Resources/AppIcon.icon  Icon Composer app icon (glyph + fill)
├── project.yml  XcodeGen spec → GitProjectsManager.xcodeproj (gitignored)
└── scripts/build-rust.sh  cargo build + uniffi-bindgen (Xcode pre-build phase)

server/          axum + SQLite sync server (kanban state; Google OAuth)
```

## Shared persistence (`dirs::config_dir()/git-projects-manager/`)

Pretty JSON, camelCase, written atomically (temp file + rename). Both apps
read/write the same files: `config.json` (folders), `settings.json`,
`kanban_v2.json`, `repos_cache_v1.json`, `remote_checks_v1.json` (gh
remote-existence debounce). Sync session in the OS
keychain (`keyring` with `apple-native`/`windows-native`/`sync-secret-service`
features; file fallback `session.json`, 0600). Kanban read-modify-write
cycles are serialized in-process; across processes files are last-writer-wins
(atomic rename prevents corruption; the next refresh + cloud sync reconciles).

## Sync configuration (build-time)

Kanban sync (both apps) reads three values from `core/src/config.rs`, each
resolved as compile-time env (`option_env!`) → dev runtime env → hardcoded
fallback: `GOOGLE_OAUTH_CLIENT_ID`, `GOOGLE_OAUTH_CLIENT_SECRET`,
`SYNC_SERVER_URL`.

A Cargo build script (`core/build.rs`) automatically loads these variables from
`server/.env` at compile time to inject them.

The **client secret fallback is empty in the public source** — no credential
ships in git; set `GOOGLE_OAUTH_CLIENT_SECRET` in `server/.env` to enable
sign-in. The client ID and server URL keep working public-endpoint fallbacks.
Desktop-client secrets are non-confidential to Google, but are still kept out
of source on principle.


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

### App icon

`Resources/AppIcon.icon` is an Icon Composer bundle (`icon.json` + `Assets/`),
not an `.icns`. XcodeGen types it as `wrapper.icon` and derives
`ASSETCATALOG_COMPILER_APPICON_NAME` from its name; `actool` compiles it into
`Assets.car` and emits a legacy `.icns` alongside, so the Info.plist key is
`CFBundleIconName` (not `CFBundleIconFile`).

The layer PNG is the **glyph alone** on a transparent canvas — no background,
no rounded tile. macOS 26 draws the tile, the `automatic-gradient` fill, the
shadow and the Liquid Glass mask itself; baking a tile into the artwork nests
it inside the system's and renders the glyph far too small. `actool`
normalizes the layer by its opaque bounding box, so the asset is cropped tight
and framed purely by `position.scale` (0.5 → the glyph spans ~61% of the tile
width, matching Apple's 824/1024 icon grid).

## Scanning

- `Scanner` (core) walks each monitored folder (walkdir, ~60 excluded dir
  names, hidden dirs skipped), detects repos by `.git/`, checks status in
  parallel with rayon (git2 for branch/dirty; `git` CLI for
  upstream/fetch/ahead/behind/remote-presence), and detects uninitialized
  sibling directories.
- `onlyLocalChecks` per folder skips fetch + ahead/behind; the `git remote`
  presence check is local, so publish state is still resolved (but never
  `RemoteNotFound`, which needs a fetch).
- **Publish-state overlays**: a `PublishState` enum (`Published` / `Unpublished`
  / `RemoteNotFound`) on each `RepoStatus` is the single source of truth;
  `categorize_results` derives two overlay vecs from it — `unpublished`
  (no remote) and `remote_not_found` (remote gone) — *in addition to* the repo's
  exclusive bucket (changes/unpushed/unpulled/clean). Errored/uninitialized
  entries are excluded; the exclusive buckets stay mutually exclusive.
- **Remote-gone detection** (online scans only): the fetch already run for
  upstream repos is classified `Reachable`/`NotFound`/`Unreachable`. A definitive
  `NotFound` is confirmed with `gh repo view` (run in the repo dir) before a repo
  is promoted to `RemoteNotFound`; any uncertainty (offline, auth, non-GitHub,
  no `gh`) stays `Published` — no false positives. The `gh` confirmation is
  debounced by `remote_checks_v1.json` (per-repo `{checked_at, exists}`,
  re-checked at most once per 24h).
- **Ordering**: statuses are sorted case-insensitively by absolute path before
  categorizing, so every `ScanResult` bucket is stable A–Z (grouped by parent
  dir). Sorting once in the core keeps both frontends identical.
- Cancellation: `Arc<AtomicBool>` polled during directory walk only; a
  cancelled `Scanner` is replaced with a fresh instance. No UI currently
  exposes cancel.
- Unborn repos (no commits) detected via typed `git2::ErrorCode::UnbornBranch`.

## Quality gates

- `just clippy` — clippy pedantic, zero warnings across `core`,
  `desktop/src-tauri`, `macos/ffi` (CLAUDE.md requirement).
- `just test` — core tests (glob matcher, fetch/`gh` reachability classifiers,
  unpublished-overlay + repo-ordering integration tests, …).
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

Local installs (a build made on this machine, not a published artifact) have
one script per platform, all with the same shape — check platform, optionally
build, stop the running instance, install: `install_arch.sh` (binary +
`.desktop` entry + icons under `/usr/local`), `install_macos.sh --build`
(`.app` → `/Applications`, ad-hoc re-signed), and `install_windows.py --build`
(Python 3, stdlib only; builds `--bundles nsis`, runs the installer with `/S`,
then confirms the install via the `com.gitprojectsmanager.app` uninstall
registry key). The NSIS bundle owns the Start menu shortcut, uninstaller and
WebView2 check, so nothing is copied by hand on Windows.
