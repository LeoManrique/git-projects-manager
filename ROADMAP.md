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
      block), silent focus-scan no longer supersedes visible scans, keyring
      `vendored` dbus for Linux builds
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

## Pending

- [ ] Re-run the multi-agent adversarial code review of the migration (first
      attempt aborted on session usage limits; a manual review pass was done instead)
- [ ] macOS app releases: signing identity + notarization (currently ad-hoc
      signed; installer strips quarantine)
- [ ] App icon asset catalog for the macOS app (currently reuses icon.icns)
- [ ] Retire the Tauri app on macOS once the native app has parity confidence
- [ ] Consider: scan-cancel UI, per-repo push action, folder reordering
- [ ] Update stale `desktop/docs/*` (SETUP paths, QUICK_START npm→pnpm)
