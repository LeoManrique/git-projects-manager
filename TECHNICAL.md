# TECHNICAL.md — Technical Specification

## Architecture

```
core/            gpm-core (Rust, edition 2024) — Tauri-free shared core
├── domain/      scan pipeline (finder → status checker → registry, rayon-parallel),
│                folder/settings/kanban/auth types
├── infrastructure/  git ops (git2 + git CLI), stores (JSON, atomic writes),
│                launcher (open in terminal/editor/URL/folder), gh CLI, OAuth PKCE,
│                sync client, keyring token store, diagnostics log (tracing)
├── services/    shared orchestration: kanban refresh/move/delete + cloud
│                sync merge, Google sign-in/out, folder edit/delete (used by
│                both frontends)
└── resources/   terminals.json / editors.json catalogs (compile-time embedded)

desktop/         Tauri 2 app — Windows/Linux
├── src/         React 19 + TypeScript + Tailwind 4 (Vite)
└── src-tauri/   thin command shim over gpm-core (#[tauri::command] wrappers)

macos/           native macOS 26+ app — full parity (kanban + sync included)
├── ffi/         gpm-ffi: UniFFI 0.32 staticlib over gpm-core
│                (proc-macro exports; async scan/git/kanban/sync on tokio,
│                streamed scans through a `FolderScan` handle)
├── generated/   Swift bindings (build artifact, gitignored)
├── GitProjectsManager/  SwiftUI (Swift 6, @Observable, Liquid Glass)
│   └── Resources/AppIcon.icon  Icon Composer app icon (glyph + fill)
├── project.yml  XcodeGen spec → GitProjectsManager.xcodeproj (gitignored)
└── scripts/build-rust.sh  cargo build + uniffi-bindgen (Xcode pre-build phase)

server/          axum + SQLite sync server (kanban state; Google OAuth)
```

## Shared persistence (`dirs::config_dir()/git-projects-manager/`)

Located by one helper (`infrastructure::app_dir::app_data_dir`, which also
creates it). Pretty JSON, camelCase, written atomically (temp file + rename). Both apps
read/write the same files: `config.json` (folders), `settings.json`,
`kanban_v2.json`, `repos_cache_v1.json`, `remote_checks_v1.json` (gh
remote-existence debounce). A kanban card's `notes` is optional and left out
of the JSON when empty, so `kanban_v2.json` keeps version 2 and a file
written before notes existed still loads. Sync session in the OS
keychain (`keyring` 4 with its default `v1` feature: the native store per
platform, zbus Secret Service on Linux so no libdbus is linked; file fallback
`session.json`, 0600). Kanban read-modify-write
cycles are serialized in-process; across processes files are last-writer-wins
(atomic rename prevents corruption; the next refresh + cloud sync reconciles).
The `logs/` subfolder holds the diagnostics log (see Logging).

## Logging (`infrastructure::logging`)

- The core owns the log file for both apps (file naming, retention and contents:
  FRONTEND.md §6.4): a `tracing` subscriber over a `tracing-appender`
  `RollingFileAppender` (`Rotation::DAILY`, `max_log_files`, prefix = the app
  name passed to `init`), with `ChronoLocal` line timestamps. Level `INFO`;
  debug builds also mirror to stderr (Xcode console, `tauri dev` terminal).
- `logging::init(app, version)` runs first thing in each app (Swift `AppLog.start()`
  from `App.init`, Tauri `main`), before `AppState`, so startup failures are
  logged. Idempotent via a `OnceLock` holding the first call's outcome. It also
  installs a panic hook that logs the panic with a forced backtrace, then chains
  to the previous hook.
- The writer is the appender itself, not `tracing_appender::non_blocking`: the
  volume is a few lines per git command, and a background writer needs a
  `WorkerGuard` that nothing in an FFI library can hold until exit, so the lines
  right before a crash would be lost.
- `tracing-subscriber` is built with `fmt` + `chrono` only. No `tracing-log`, so
  records from the `log` crate (Tauri's own) are not captured; no `ansi`, and the
  file layer sets `with_ansi(false)` anyway since features merge across a build.
- Frontend lines enter through `logging::frontend(level, message)` (target
  `frontend`): FFI `log_message` (UniFFI enum `LogLevel`), Tauri command
  `log_message` (serde lowercase `"error" | "warn" | "info"`). All free text —
  frontend messages, git stderr, repo errors, panic messages and backtraces —
  goes through `log_safe`: the userinfo of any `scheme://user:token@host` URL is
  masked to `***` (git can echo a remote URL with its token), and CR/LF become
  `\r`/`\n` so one event is one line.
- `logs_folder` / `get_logs_folder` return the path; `open_logs_folder` opens it
  with the platform opener (`launcher::open_path`, the same `open` / `xdg-open` /
  `cmd /C start` helper `open_url` uses).

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

## Sync server (`server/`)

axum 0.8 on tokio; SQLite through `rusqlite` (bundled, WAL) behind an `r2d2`
pool. The schema is created at start with `CREATE TABLE IF NOT EXISTS`, and a
column added after the first release gets its own guarded step: `notes` on
`manifest_cards` is added with `ALTER TABLE` only when `pragma_table_info`
does not list it. Sign-in verifies a Google ID token with `jsonwebtoken`
against Google's JWKS, fetched by `reqwest`; both run on rustls with the
`aws-lc-rs` provider, and trust roots come from the OS store
(`rustls-platform-verifier`), which is why the runtime image installs
`ca-certificates`. Sessions are opaque UUID tokens with a TTL
(`SESSION_TTL_DAYS`). `POST /v1/sync` takes the client's cards and answers
with every stored card of that user; a stored card's column, notes and
`updated_at` are replaced only by a strictly newer `updated_at`, so moves and
notes merge per card, last writer wins. The crate is edition 2021 with
`rustfmt.toml` set to `style_edition = "2024"`, so its layout matches `core/`.


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
- `[profile.release.build-override] strip = "none"` in `macos/ffi`: under
  Xcode's `MACOSX_DEPLOYMENT_TARGET`, a stripped proc-macro dylib fails to load
  and the Release build stops with E0463. The shipped staticlib is unaffected.

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
  parallel with rayon, and — when the folder asks for it — detects uninitialized
  sibling directories.
- **Walk order**: `.git` and hidden directories are pruned first, then the
  directory is tested for `.git/`, and only a non-repo is pruned by an excluded
  name. Testing last made a repo the user named `build`, `dist`, `packages`,
  `public`, `bin`, `gen` or `out` invisible — pruned on its name with no entry
  and no error. The `NESTED_EXCLUDED_DIRS` rule still wins inside a repo even
  for a repo: `<repo>/lib` is vendored by definition, which is what it hides.
- Monitored folders may not overlap; `ConfigManager` rejects an add or edit
  whose path equals, contains, or sits inside another folder's. Compared by
  path component (so `/a/bc` is not inside `/a/b`) after `canonicalize` when
  both exist. Two overlapping folders would scan shared repos concurrently,
  racing two `git fetch` processes in one `.git`.
- **One libgit2 handle per repo** (`RepoInspector`): branch, dirty state,
  remote presence and ahead/behind all come from a single `Repository::open`.
  Only `fetch`, `pull` and `clean` still spawn `git`. The four local
  subprocesses this replaced cost 2.86 s of CPU across 70 repos against 19 ms
  for the libgit2 equivalents — both `git log <range> --oneline` calls ran a
  full revwalk and formatted output that was discarded, since only its
  emptiness was read.
- **Fetch/ahead-behind ordering**: `upstream_ref()` (which requires the tracking
  ref to *resolve*, as `git rev-parse @{upstream}` did) decides whether to
  fetch; `ahead_behind()` runs after it. The handle is re-opened in between —
  libgit2 re-reads refs and objects per lookup, but loads the shallow-clone
  boundary once at open time. If `graph_ahead_behind` errors (replace refs, a
  commit-graph over a shallow boundary, a non-UTF-8 refname), one
  `git rev-list --left-right --count` answers instead, so a libgit2 gap costs a
  subprocess rather than silently reporting the repo as Clean.
- **Scan thread pool**: status checks run on a dedicated rayon pool
  (`4 × CPUs`, clamped to 8–32) rather than the global one. Almost all of a
  check's wall time is a `git fetch` blocked on DNS/TLS, so a CPU-sized pool
  serializes the fetches into `repos / CPUs` waves. Kept off the global pool so
  no other rayon user inherits a thread count sized for blocking I/O. Each repo
  is one job spawned through `in_place_scope` from the calling thread, which is
  outside the pool and simply blocks. The `par_iter` it replaced joined on a
  pool thread, and a joining thread runs other queued jobs while it waits, so
  one folder's scan finished only after another folder's slow repo.
- **Open files limit** (`resource_limits`): `AppState::new` raises the soft
  `RLIMIT_NOFILE` to `min(10240, hard)` (macOS rejects more than `OPEN_MAX`). A
  macOS app starts at 256, which a Clean All during a rescan exhausted ("Too many
  open files"). No-op on Windows.
- **`git` invocation invariants** (`git_command()`): `core.quotePath=false` (git
  otherwise C-quotes non-ASCII paths, which broke the `git clean` parser),
  `LC_ALL=C` (output we match on stays English), `GIT_TERMINAL_PROMPT=0` (a
  credential prompt would block forever).
- **One runner** (`run_git`): every git subprocess goes through it, under a
  wall-clock timeout, and it logs the outcome — failure or timeout at `WARN`
  with the repo, the arguments minus the constant `-c` knobs, the duration, the
  exit status and the whole stderr; a slow success (`SLOW_GIT`) at `INFO`; other
  successes at `DEBUG` (not recorded at the `INFO` level). The one failure it
  cannot see, a clean whose deletions partly fail, is logged by `clean` itself
  (`clean incomplete`).
- **Network limits** (bound a stalled transfer; libcurl's default connect
  timeout is 300 s): `limit_http` (`fetch` and `pull`) sets
  `http.lowSpeedLimit=1000` + `http.lowSpeedTime=20`; `limit_ssh` (`fetch` only)
  sets `GIT_SSH_COMMAND` to `ssh` with `ConnectTimeout=10`/`BatchMode`, but only
  when none of `GIT_SSH_COMMAND`, `GIT_SSH` or the repo's `core.sshCommand` (read
  through libgit2's config levels) is set — `GIT_SSH_COMMAND` outranks the other
  two, so setting it replaced per-account key setups. Pull never gets it:
  `BatchMode` also disables the passphrase prompt a user-started pull may need.
- **Fetch flags**: `gc.auto=0` + `maintenance.auto=false` (no per-repo background
  repack fork), `--no-tags --no-recurse-submodules` (nothing in a scan result
  uses them). `pull` does not pre-fetch — `git pull` is fetch + merge — and runs
  `--quiet`, so its stderr holds only the error: the progress lines ("From
  <url>", "* [new tag]") used to fill the two lines the error banner shows.
- **Fetch debounce**: a successful fetch is recorded per repo in a process-wide
  map, and a fetch within 30 s of it is skipped (reported `Reachable`, since that
  is what the skipped fetch established). Ahead/behind is still computed from the
  tracking refs, so counts stay current to within the window — only the round
  trip goes. Scans arrive in bursts (post-action recheck, focus rescan after
  startup) that repeat identical network work; a repeat scan measures
  **3.4 s → 1.0 s** over 76 repos. Failures are never recorded, so an unreachable
  remote is retried immediately. A successful `pull` records too: its fetch just
  updated the tracking refs the recheck after it reads.
- **Wall-clock timeouts** (`infrastructure::process`): `git fetch` is killed
  after 20 s, `git pull` after 5 min, the local `git rev-list` and `git clean`
  dry run after 1 min, and `gh` after 15 s (60 s for `repo list`). git's own
  knobs bound a *stalled transfer* but not a TCP connect to a black-holed route,
  and `gh` has no equivalent knob. The pull limit is generous because a pull
  killed mid-merge can leave `.git/index.lock` behind. The helper drains stdout
  and stderr on their own threads — `try_wait` never reads the pipes, so a child
  filling the 64 KiB pipe buffer would deadlock against the loop timing it out.
  On Unix the child leads its own process group (`process_group(0)`), and a
  timeout sends the group `SIGTERM` (git removes its lock files on it), waits up
  to 1 s, then `SIGKILL`: git's `ssh`/`git-remote-https` helpers inherit its
  stderr, so killing git alone left the reader threads waiting on them for up to
  libcurl's 300 s connect timeout. A descendant that leaves the group is bounded
  too: once the child exits, the pipes get 2 s (`PIPE_GRACE`) and are then
  abandoned. A killed fetch is an error, so it never reads as `NotFound`.
- `onlyLocalChecks` per folder skips fetch + ahead/behind; the `git remote`
  presence check is local, so publish state is still resolved (but never
  `RemoteNotFound`, which needs a fetch).
- **Overlay categories**: three vecs sit *in addition to* the repo's exclusive
  bucket (changes/unpushed/unpulled/clean). A `PublishState` enum (`Published` /
  `Unpublished` / `RemoteNotFound`) drives the first two — `unpublished` (no
  remote) and `remote_not_found` (remote gone). The third, `remote_state_unknown`,
  comes from a `bool` on `RepoStatus` and is independent of publish state.
  Errored/uninitialized entries are excluded from all three; the exclusive
  buckets stay mutually exclusive.
- **`remote_state_unknown`** separates "asked and failed" from "did not ask".
  Both were previously `None`, and `None` falls through to Clean — so a repo
  whose comparison failed was asserted to have nothing to push. It is set only
  when both the libgit2 walk *and* the `git rev-list` fallback fail, and is
  always `false` under `only_local_checks`, where not asking is the configured
  behavior rather than a failure.
- **Remote-gone detection** (online scans only): the fetch already run for
  upstream repos is classified `Reachable`/`NotFound`/`Unreachable`. A definitive
  `NotFound` is confirmed with `gh repo view` (run in the repo dir) before a repo
  is promoted to `RemoteNotFound`; any uncertainty (offline, auth, non-GitHub,
  no `gh`) stays `Published` — no false positives. Only GitHub's own wording
  ("could not resolve to a repository", "repository not found") counts as
  `NotFound`; a shell's "command not found" explicitly does not.
  - **Gated on `gh` being authenticated**, resolved once per scan and lazily.
    GitHub answers 404, not 403, for a private repo you cannot see, and `git
    fetch` says "not found" for the same reason — so both confirmations agree
    wrongly the moment credentials expire. Checked *before* the cache, since a
    verdict recorded while unauthenticated is the one not to trust.
  - Debounced by `remote_checks_v1.json` (per-repo `{checked_at, exists}`,
    re-checked at most once per 24 h). Persisted by a locked read-merge-write
    (newest `checked_at` wins) because both apps and every folder scan write the
    same file — `write_atomic` prevents corruption, not lost updates. Entries
    whose path no longer exists are pruned on load and on save.
- **`gh` invocation**: the binary is located once per process through a login
  shell (a GUI app inherits no PATH), which also captures `GH_TOKEN`,
  `GITHUB_TOKEN`, `GH_HOST` and `GH_CONFIG_DIR`; every later call execs it
  directly with argv. Re-sourcing the login profile per call cost ~52 ms and
  made behavior depend on how the app was launched.
- **Ordering**: statuses are sorted case-insensitively by absolute path before
  categorizing, with a case-sensitive tie-break so the comparator is a total
  order (without it, paths differing only in case fall back to the registry's
  hash-map order, which varies between runs). Every `ScanResult` bucket is a stable A–Z grouped
  by parent dir; sorting once in the core keeps both frontends identical.
- **Clean** (`git clean -fdXn` dry run, filtered in Rust, survivors deleted): a
  path that fails to delete no longer aborts the repo. An already-gone path is
  treated as success (a build or watcher can remove it between the dry run and
  the delete); real failures are collected and the error names every one.
- **Uninitialized detection** walks siblings of discovered repos with
  `DirEntry::file_type` (no syscall, does not follow symlinks), matching
  `RepositoryFinder`'s `follow_links(false)`. Following symlinks let a link back
  to an ancestor report the same folder once per level until the OS refused the
  chain. It is a second full walk of the tree, and the folder's
  `detect_uninitialized` flag skips it outright — a general-purpose folder
  reports every ordinary directory otherwise. The flag reaches the core as a
  `scan_folder` parameter, and `#[serde(default = "enabled")]` makes it `true`
  for folders stored before it existed, so an upgrade changes nothing.
- **Registry** (`scanner/registry.rs`): the `Scanner` keeps each folder's
  latest state. A scan seeds it after the walk (repos no longer found are
  dropped, every found repo is `pending`, repos with no previous status are in
  `checking`) and applies each status as it is read; every change returns a
  whole, categorized `ScanResult` with an increasing `revision` shared by every folder. A
  status is kept only if no later-started read was applied, so a slow read
  never overwrites a fresher one. `finish` completes only the newest scan of
  the folder. Editing a folder's path or deleting it (`services::folders`)
  forgets its state.
- **Streaming**: `scan_folder_streaming` hands each snapshot to a callback on
  the calling thread (the walk's, then one per landed repo, skipping any that
  arrive after a newer one) and returns the complete one; `scan_folder` is it
  without the callback. `recheck_repos` re-reads repos after an action and
  skips those a scan in flight has queued but not started, since that scan
  reads them fresh. The bridges carry the snapshots: Tauri's `scan_folder`
  sends them through an `ipc::Channel` and still returns the final one;
  macOS's `start_folder_scan` returns a `FolderScan` whose async `next()`
  reads a tokio `watch` channel, which keeps only the newest snapshot, so a
  busy main actor skips the ones in between. A Swift callback would have run
  Swift code on the scan's threads. Both bridges export `recheck_repos`.
  Each frontend keeps one scan per folder (a second request joins it) and
  drops a snapshot whose `revision` is not newer than the one shown; Tauri
  applies streamed snapshots once per animation frame.
- No cancellation. The removed flag was polled only by the directory walk, so it stopped the
  cheap half and left every `git fetch` running, and it returned a `ScanResult`
  indistinguishable from a complete one that the frontends stored as
  authoritative. A cancel UI needs polling in the status loop and a partial-result
  marker first.
- Unborn repos (no commits) detected via typed `git2::ErrorCode::UnbornBranch`.
- **Scan logging**: a scan logs `scan started` and `scan finished` (repo,
  error and unknown-remote counts, seconds) per folder, so a hang shows as a
  start with no finish. Each repo that lands in Errors (`StatusChecker::failed`)
  and each `remote_state_unknown` repo is logged with its error.

## Quality gates

- `just clippy` — clippy pedantic, zero warnings across `core`,
  `desktop/src-tauri`, `macos/ffi`, `server` (CLAUDE.md requirement).
- `just test` — core tests: glob matcher, fetch/`gh` reachability classifiers,
  folder-overlap and config-upgrade unit tests, subprocess-timeout unit tests
  (including a timed-out child whose grandchild holds the pipes), log-line
  folding and git argument description,
  and integration tests for the unpublished overlay, repo ordering, clean paths,
  repo folder names, symlinked and opted-out uninitialized folders, and
  ahead/behind (a local bare repo stands in for the remote, so the whole suite
  is offline). Server tests: the `notes` column migration on fresh and
  pre-notes databases, and per-card last-writer-wins for notes through the
  sync handler, each against a throwaway SQLite file (no network).
- `just check-desktop` — frontend gates: `pnpm build` (tsc strict + vite) then eslint.
- `just bench-scan <path> [local]` — times three `scan_folder` runs and prints
  every bucket count, so a scanner change can be shown to be faster *and* to
  still find the same repos.

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
