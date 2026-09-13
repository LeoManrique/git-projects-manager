# ROADMAP.md

## Done

- [x] Tauri app (Windows/Linux/macOS): folders, scanning, settings, kanban + sync (v2.2.0)
- [x] Extract Tauri-free `core/` crate (`gpm-core`) shared by all frontends
  - [x] Atomic JSON writes; typed UnbornBranch detection; shared launcher module
  - [x] Fix keyring: platform features (`apple-native`, …) so sessions actually persist
  - [x] clippy pedantic: zero warnings across all Rust crates
- [x] `FRONTEND.md` — behavior source of truth for both frontends
- [x] Native macOS app (`macos/`): SwiftUI (macOS 26+, Liquid Glass, @Observable)
  - [x] `gpm-ffi` UniFFI bridge (async scan/pull/clean over tokio)
  - [x] Sidebar navigation, All Folders overview (all sections expanded
        inline, Clean last), per-folder detail sections, repo actions,
        folder CRUD, Settings scene (Default Apps, Git Clean), search,
        auto-scan + focus rescan + supersession
- [x] Tauri UI aligned with the macOS app: sidebar navigation (All Folders,
      Kanban, folders + attention badges, Add Folder/Settings footer), All
      Folders overview with sections expanded inline, per-folder detail view,
      macOS-style repo rows
- [x] Single-line repo rows in both apps (muted directory + emphasized name
      + branch chip) with filename-first path truncation ported from leogit's
      `PathText`: directory shrinks to a `…/` hint before the name
      middle-truncates
- [x] Docs: README, DESIGN, TECHNICAL, FRONTEND; justfile recipes (`dev-macos`, `clippy`, `test`)
- [x] Post-migration hardening: per-process unique temp names for atomic store
      writes (two apps share the files), async FFI `cancel_scan` (no main-thread
      block), the focus rescan never supersedes an in-flight scan, keyring
      `vendored` dbus for Linux builds
- [x] Focus rescans are no longer silent: the window-focus (and post-action)
      rescan runs as a full scan in both apps, showing the same global +
      per-folder progress as Scan All (still 20s-throttled)
- [x] macOS releases ship the native SwiftUI app: `deploy_releases.sh` builds
      and uploads it (version bump covers `project.yml`), `install_release.sh`
      installs it (macOS 26+ check, replaces older Tauri installs)
- [x] Kanban board + cloud sync in the native macOS app (full parity):
      orchestration hoisted into `gpm-core::services` (shared with the Tauri
      commands, DRY), kanban/gh/auth/sync exported over UniFFI, SwiftUI board
      (native drag & drop, sync status chip, Account settings tab), and the
      Tauri board restyled to match the new design (tinted column headers,
      count capsules, sync chip, named relative dates; dead `review` column
      id removed)

- [x] Unpublished (no-remote) category as an overlay in both apps: a repo with
      no remote appears in `ScanResult.unpublished` on top of its primary
      section (blue, no Fetch & Pull); backed by a `hasRemote` flag on
      `RepoStatus` and a core integration test

- [x] `scripts/install_windows.py` — local-build installer for Windows, matching
      the arch/macOS ones (build `--bundles nsis`, stop running instance, silent
      `/S` install, registry-verified)

- [x] Repo row/context actions: added **Copy Path** in both apps, and gave every
      category a stable case-insensitive alphabetical-by-path order (sorted once
      in the core, shared by both frontends; covered by a core integration test)

- [x] **Remote Not Found** overlay (both apps): repos whose configured remote was
      deleted on the host no longer masquerade as Clean. A `PublishState` enum
      (`Published`/`Unpublished`/`RemoteNotFound`) replaces the old `hasRemote`
      flag as the single source of truth; detection needs `git fetch` to report
      "not found" **and** a `gh` confirmation (debounced once/24h via
      `remote_checks_v1.json`), so offline/auth/non-GitHub cases never false-
      positive. Pink section, no Fetch & Pull; classifiers unit-tested

- [x] macOS app icon: Icon Composer `AppIcon.icon` (glyph-only layer, system
      draws tile/fill/mask) replaces the padded `.icns` that rendered small

- [x] Scanner performance & reliability pass (`docs/SCANNER_PERFORMANCE.md`):
      online scan of 76 repos **4.7s → 2.3s** via a dedicated oversubscribed
      scan thread pool (the old CPU-sized rayon pool serialized the fetches);
      fetch hardened (`gc.auto=0`, `--no-tags`, HTTP/SSH stall timeouts); `pull`
      no longer double-fetches; pull/clean rescan only the affected folders.
      Correctness: `git clean` handles non-ASCII paths (`core.quotePath=false`)
      and no longer aborts a whole repo on one unremovable path; a `gh` missing
      from `PATH` no longer reads as "repository deleted"; failed `git remote` /
      `git log` no longer return a confident wrong answer. Frontends: Tauri
      streams per-folder results and finally shows bulk action messages, both
      apps report the first failure reason, `spawn_blocking` for the Tauri
      commands. New `just bench-scan` + `core/tests/clean_paths.rs`

- [x] Scanner Tier 2 (`docs/SCANNER_PERFORMANCE.md`): one libgit2 handle per repo
      replaces the four local `git` subprocesses (2.86s of CPU → 19ms over 70
      repos; local-only scan **0.66s → 0.46s**, online unchanged because it is
      fetch-bound); wall-clock timeout + kill on `fetch`/`gh`; `RemoteNotFound`
      gated on one lazy `check_auth()` per scan (GitHub 404s private repos you
      cannot see, so both "independent" confirmations agreed wrongly once
      credentials expired); remote-check cache persists by locked
      read-merge-write and prunes dead paths; `gh` resolved once and exec'd with
      argv instead of a login shell per call; the uninitialized walk no longer
      follows symlinks (the same folder was reported 16 times through a cycle).
      An adversarial review of the libgit2 swap added a post-fetch re-open and a
      `git rev-list` fallback so a libgit2 gap costs a subprocess instead of
      silently reading as Clean. 14 new tests, all offline

- [x] Scanner Tier 3, from the three product decisions:
      **nested repos stay supported** (no walk pruning); **fetch stays on every
      scan** but a successful one is debounced per repo for 30s, so the bursts
      (post-action rescan, focus rescan after startup) reuse it — a repeat scan
      of 76 repos goes **3.4s → 1.0s**; and a repo whose remote comparison was
      *attempted and failed* now lands in a new **Unknown Remote State** overlay
      instead of falling through to Clean. Paired with it, a **"Local checks
      only" chip** beside Scan All in both apps, since `onlyLocalChecks` folders
      report no unpushed/unpulled work only because they never asked — that is
      "did not check", which the Unknown section deliberately does not claim

- [x] Scanner correctness guards, closing `docs/SCANNER_PERFORMANCE.md`: a repo
      is no longer hidden by its own folder name (the excluded-name prune ran
      before the repo check, so seven repos named `build`, `dist`, `packages`, …
      were found **zero** times by the old code); overlapping monitored folders
      are rejected instead of scanning shared repos twice with two `git fetch`
      processes racing in one `.git`; and the unusable `cancel_scan` is deleted
      — it was exposed through both frontends with no callers, cut only the walk
      and not the fetches, deadlocked against the scan it cancelled, and returned
      a partial result the frontends stored as complete. Both folder forms now
      show the core's reason instead of a generic failure. 11 new tests

- [x] One scan control per view, in both apps: the toolbar/header button now
      targets what is on screen (**Scan All** in the overview, **Scan Folder**
      in a folder's detail view) and ⌘R runs the same action, replacing the two
      side-by-side buttons on macOS and a Tauri header that scanned every folder
      while a single one was open. Extracted as a shared `ScanButton` in both
      apps, reading a single `foldersInView` the local-checks chip also uses.
      macOS registers a 350 ms `NSInitialToolTipDelay` so toolbar help text
      stops taking seconds to appear

- [x] Per-folder **"Only code projects"** switch (default on): with
      it off the scan skips the uninitialized walk entirely, so a general-purpose
      folder (Documents) no longer reports every ordinary directory as a project
      you forgot to `git init` — the noise was burying its actual repos. Missing
      from an older `config.json` reads as on, so upgrades change nothing. The
      Tauri form now passes one values object instead of positional booleans

- [x] Sync server brought to current dependencies (axum 0.8; rustls with
      `aws-lc-rs` and the OS trust store for the Google JWKS fetch and token
      checks) and into `just test` / `just clippy`, with its first unit tests

- [x] Per-card **notes** on the kanban board, back in both apps and synced
      (dropped in the move to GitHub-backed cards): optional `notes` on the
      card, left out of `kanban_v2.json` when empty so the file stays v2; a
      store/service `set_notes` sharing the move's mutate-then-sync path;
      the server column added by a guarded `ALTER TABLE` and replaced whole
      on upsert; a Tauri command and a UniFFI export; and an inline quick
      editor on the card in both apps (*Add/Edit Notes…* menu entry, blur
      saves, Escape cancels, no drag while editing, untouched drafts never
      overwrite a synced edit). The v1 notes were exported to a text file for
      manual re-entry rather than imported. Core, merge and server tests

## Pending
- [ ] Re-run the multi-agent adversarial code review of the migration (first
      attempt aborted on session usage limits; a manual review pass was done instead)
- [ ] macOS app releases: signing identity + notarization (currently ad-hoc
      signed; installer strips quarantine)
- [ ] Retire the Tauri app on macOS once the native app has parity confidence
- [ ] Consider: per-repo push action, folder reordering, and a scan-cancel UI —
      which needs the flag polled in the status loop (not just the walk) and a
      partial-result marker, since the old entry point had neither
- [ ] Update stale `desktop/docs/*` (SETUP paths, QUICK_START npm→pnpm)
