# Plan: streamed scans, a "Last scan" indicator and a calmer schedule

Status: slices 1–2 of 8 done (the core). Next: slice 3, the bridges. The "why"
is in `docs/analysis/automatic-scan-strategy.md`. Delete this file once the
manual test script at the bottom passes.

## Decisions

| Question | Decision |
|---|---|
| Automatic triggers | Launch (as today), a silent background scan every 10 min, and a focus scan only when the last scan started 15+ min ago. |
| What the clock measures | Time since the **start** of the last full scan. A manual Scan All counts, so pressing it also pushes the next automatic scan back. |
| Short absences | Do not count. The 15 min rule is the only debounce. |
| What a scan refreshes | Local and remote state, every repo of every folder. No special handling for slow or failing repos. |
| UI during a visible scan | Today's indicators, plus a spinner on every repo still being checked. Results stream in per repo. Folder headers and sidebar rows keep their counts and badges, updating live, with a spinner beside them instead of in their place. |
| Timer while focused | Runs. A focused window that never refreshes would go stale in plain sight. |
| What moves the global clock | Only scans that cover every folder. A per-folder *Scan Folder* refreshes one folder, so it does not reset a label that speaks for all of them. Post-action rechecks are repo-level and never count. |
| New repos during a scan | A transient *Checking* section, only for repos with no previous status. Repos already shown stay in their old section with a spinner until their check finishes. |
| "Last scan" indicator | One global label, relative ("3 minutes ago") with the absolute time in a tooltip. |
| Its timestamp | The scan's start, provided by the core in `ScanResult`. A scan whose fetches failed still counts. |
| Re-ticking | Only when the text would change: often while recent, rarely when old, and at once on refocus. |
| Persisting results | Not done. Streaming makes the empty launch short enough. |
| Background scanning | While a window exists: focused, unfocused or minimized. No App Nap assertion: if the OS throttles the app, the scan simply does not happen. |
| Background cadence | Fixed 10 min, not configurable. |
| Background results | Replace the screen silently: no indicators, no spinners. |
| Overlap | Never two scans of one folder. A request for a folder already scanning joins the scan in flight. |
| Diagnostics | No new logging. |

## User-facing behavior (goes into FRONTEND.md §3 and §5)

- **Launch.** A visible full scan starts as today. Each folder lists its repos
  under *Checking* with a spinner as soon as the walk finishes (well under a
  second), and each repo moves into its section the moment its check ends.
- **Visible scan** (launch, Scan All / ⌘R, Scan Folder, focus): the toolbar
  button reads "Scanning…" as today, and every repo row still being checked
  shows a spinner. Rows that were already in a section stay where they are,
  with a spinner, until their new status moves them.
- **Folder header and sidebar row during a visible scan**: the header keeps
  "{n} repos" and the clean badge, the sidebar row keeps its attention badge,
  both updating as repos finish, with a small spinner beside them. Today they
  are replaced by "Scanning…" / a spinner until the whole folder is done.
  Before a folder's first result (launch), the header reads "Scanning…" as
  today, since there is nothing to count yet.
- **Silent scan** (the 10 min timer): no indicators of any kind. Rows move
  between sections as their new status arrives. The *Checking* section stays
  hidden; a new repo appears once checked. A snapshot equal to what is on
  screen publishes nothing.
- **Joining.** Pressing Scan All during a silent scan does not start a second
  one: the folders already scanning switch to visible (indicators and
  spinners appear for the repos still pending), and only idle folders start.
- **Focus.** Returning to the app scans only if the last full scan started
  15+ min ago. With the timer at 10 min this only happens when the OS held the
  timer back (App Nap, sleep, a throttled WebView), which is exactly when the
  screen may be stale.
- **After a pull or clean.** Only the affected repos are rechecked, not their
  whole folder. The row keeps its spinner from the action through the
  recheck. Pull All / Clean All recheck exactly the repos they touched. A
  recheck never starts a folder scan and never moves the clock.
- **"Last scan" label.** Next to the scan button in both apps, on All Folders
  and folder detail (not on the board): "Last scan: 3 minutes ago", tooltip
  with the absolute date and time. Hidden until the first full scan starts.
- **Vocabulary** (leogit's, so the two projects read the same): "just now"
  under a minute, then "N minute(s) ago", "N hour(s) ago", "N day(s) ago",
  "N month(s) ago" (30-day months), "N year(s) ago". Floor rounding, singular
  at 1. Tooltip: macOS `.abbreviated` date + `.shortened` time; Tauri
  `toLocaleString(undefined, { dateStyle: 'medium', timeStyle: 'short' })`.

## Data flow

```
frontend scheduler ── one request per folder ──▶ in-flight map (frontend)
                                                     │ (join if present)
                                                     ▼
core Scanner::scan_folder_streaming ── walk ──▶ FolderState (core registry)
        │                                         ▲   seeds pending repos with
        │  one pool job per repo                  │   their previous status
        └─▶ StatusChecker::check ── apply ────────┘
                                     │ categorize, bump revision
                                     ▼
                     on_snapshot(ScanResult), on the calling thread
                     ┌───────────────┴───────────────┐
              macOS: tokio watch               Tauri: ipc::Channel
              → FolderScan.next() async        → onmessage, flushed
              (keeps only the newest)            once per animation frame
                     └───────────────┬───────────────┘
                           results[folderId] = snapshot
                           (ignored if revision is older)
```

Categorizing and sorting stay in the core, so neither frontend learns the
bucket rules: every event is a whole, categorized `ScanResult`, and the
frontends keep doing what they do today with it (replace the folder's entry).

## Changes by layer, core outward

### 1–4. Core (done)

TECHNICAL.md (Scanning → Scan thread pool, Registry, Streaming, Fetch
debounce) and FRONTEND.md's data model describe it. What the bridges need:

- `Scanner::scan_folder_streaming(path, only_local_checks,
  detect_uninitialized, on_snapshot: impl FnMut(ScanResult)) -> ScanResult`
  blocks until every repo is checked. `on_snapshot` runs on the calling
  thread with every snapshot but the final one, which is returned. A closure
  replaced the planned `ScanSink` trait: it needs neither `Send` nor `Sync`.
- `Scanner::recheck_repos(folder, &[PathBuf], only_local_checks) ->
  Option<ScanResult>`, blocking; `None` for a folder never scanned.
- Call both from outside `SCAN_POOL` (`spawn_blocking` is fine): the caller
  blocks while the jobs run on the pool.
- Until slice 4, two scans of one folder can still overlap; the older one
  then returns a snapshot with `is_complete == false`.
- `pull()` calls `record_fetch`, so a recheck within 30 s reads the refs the
  pull updated.
- Found on the way: with more queued repos than pool threads, a folder's jobs
  still queue behind another folder's; they wait for those to *start*, no
  longer to finish.
- Found in slice 1: the macOS Release build failed with E0463 because a
  stripped proc-macro dylib does not load under Xcode's
  `MACOSX_DEPLOYMENT_TARGET`. Fixed with `[profile.release.build-override]
  strip = "none"` in `macos/ffi/Cargo.toml`.

### 5. UniFFI bridge: `macos/ffi/src/lib.rs` (UniFFI 0.32.2)

- `#[derive(uniffi::Object)] FolderScan` wrapping a `tokio::sync::watch`
  receiver, with `async fn next(&self) -> Option<ScanResult>`: waits for a
  newer snapshot, returns `None` after the complete one has been returned.
  A watch channel keeps only the newest value, so a busy main actor skips
  intermediate snapshots instead of queueing them.
- `GpmCore::start_folder_scan(...) -> Arc<FolderScan>`: spawns the blocking
  streaming scan with a callback that `send`s into the watch sender, then
  sends the returned final snapshot.
- `GpmCore::recheck_repos(...)`: async, `spawn_blocking`, like `pull_repo`.
- Chosen over a foreign-trait callback because the callback would run Swift
  code on rayon workers; the object keeps every Swift call on the async side
  the bridge already uses. Fix the stale `:583` comment
  ("`git fetch` + `git pull`").

### 6. Tauri commands: `desktop/src-tauri/src/commands/scan.rs` (Tauri 2.11.6)

- `scan_folder` gains `on_snapshot: tauri::ipc::Channel<ScanResult>`, which
  the callback `send`s to. The command still returns the complete result.
- New `recheck_repos` command, registered in `main.rs`.
- `desktop/src/lib/api.ts`: `scanFolder(folder, onSnapshot)` creates the
  `Channel`; `recheckRepos(folder, paths)`.

### 7. Scan coordinator, both frontends

Replaces the version counter and supersession (FRONTEND.md §5.2): with one
scan per folder there is nothing to supersede.

- **In-flight map** `folderId → { visible: Bool }`, owned by `AppModel` /
  `useScanner`. `scanningFolders` (what drives today's indicators) becomes
  the ids whose entry is visible, so every existing indicator keeps working.
- **`requestScan(folders, visible)`**: for each folder, if in flight, upgrade
  to visible when asked; otherwise start it. One function for every trigger.
- **Applying a snapshot**: drop it if the folder is gone or its `revision` is
  not newer than the one on screen, and skip the write when it equals what is
  shown (no needless re-render). Tauri keeps the latest snapshot per folder in
  a ref and flushes once per `requestAnimationFrame`; macOS gets coalescing
  from the watch channel.
- **Global clock** `lastFullScanStartedAt`: set from the first snapshot's
  `started_at_ms` of a scan that covers every folder (the earliest across
  them). Feeds the label and both automatic rules. A full request that only
  joins scans already in flight does not move it.
- **Rechecks after actions**: `pull`, `clean`, `pullAll`, `cleanAll` call
  `recheckRepos` grouped by owning folder (`foldersForRepos` already does the
  attribution) and keep the repo flagged until it returns. This closes the
  ROADMAP item about post-action rescans overlapping a scan in flight.
- **Per-repo spinner**: a row shows a spinner when its path is in the
  folder's `pending` and the folder is in flight visibly, reusing the
  pull/clean spinner and the existing busy check (`isBusy(repoPath:)` /
  `RepoActionHandlers`). Actions on a pending row stay enabled, as they are
  for every row during today's scans.
- **Folder header and sidebar row**: `FolderSummaryHeader` and the sidebar
  folder row show their counts whenever a result exists, and add the spinner
  next to them while the folder is in flight visibly, instead of choosing
  between the two (macOS `AllFoldersView.swift` / `SidebarView.swift`,
  Tauri `AllFoldersOverview.tsx` / `Sidebar.tsx`).
- **Checking section**: a new `RepoCategory` / `SECTIONS` entry mapping
  `result.checking`, first in order, hidden when empty or when the folder's
  scan is silent.
- Tauri: extract the spinner span copied 7 times into one `Spinner`
  component, since the row spinner adds an eighth.

### 8. "Last scan" label, both frontends

- **Helper** `relativeAge(date, now) -> (text, nextChangeAt)`: the vocabulary
  above, plus the instant the text next changes (the next minute boundary
  under an hour, the next hour boundary under a day, and so on). macOS:
  `Models/RelativeAge.swift`; Tauri: `lib/relativeAge.ts`. The kanban's own
  day-only formatters stay as they are.
- **View**: macOS a `.secondaryAction` toolbar item next to `ScanButton`
  (where `LocalChecksChip` sits); Tauri a chip next to `ScanButton` in the
  `App.tsx` header. Hidden on the board and before the first full scan.
- **Ticking**: one timer that sleeps until `nextChangeAt`, re-armed when the
  clock changes and immediately on app focus / visibility. macOS: a
  `.task(id: lastFullScanStartedAt)` loop with `Task.sleep(until:)`; Tauri: a
  `setTimeout` chain plus `visibilitychange` / `focus` listeners. Under a
  minute it ticks once (at 60 s), then once a minute, then once an hour.

### 9. Scheduler, both frontends

- **One owner per app.** macOS: move the activation observer from the
  per-window `.onReceive` in `App.swift` into `AppModel` (registered once in
  `start()`), so two windows no longer double-scan. It keeps calling
  `kanban.appDidBecomeActive()`. Tauri: the `focus` listener in `useScanner`
  registers once and reads state through refs instead of re-registering on
  every scan-state change.
- **Focus rule**: `now − lastFullScanStartedAt ≥ 15 min` → visible full
  request. No other guard: joining handles anything in flight.
- **Timer**: a self-arming loop, armed for `lastFullScanStartedAt + 10 min`
  and re-armed after each run settles (leogit's `pacedLoop` pattern, so runs
  never overlap). When it fires: silent full request. macOS: a `Task` loop
  with `Task.sleep(until:)` owned by `AppModel`; Tauri: a `setTimeout` chain
  in `useScanner`. No App Nap assertion and no Rust-side ticker: a throttled
  timer fires late or not at all, and the focus rule covers the return.
- **Constants** next to each other in each app: `BACKGROUND_SCAN_INTERVAL` =
  10 min, `FOCUS_SCAN_MIN_AGE` = 15 min. The 20 s throttle goes away.
- **No window (macOS)**: the timer keeps running in `AppModel` only while a
  window exists; with none, it parks until one opens.

### 10. Docs

- **FRONTEND.md**
  - §2: results are still session memory; add the clock as session memory.
  - §3: auto-scan row, new "Last scan" row, scheduler rows.
  - §5.1: triggers rewritten (launch, timer, focus 15 min, manual), visible vs
    silent, joining, streaming.
  - §5.2: supersession replaced by "one scan per folder".
  - §5.3: per-repo spinner, Checking section, live counts with a spinner in
    the folder header and sidebar row.
  - §5.5: post-action recheck of the affected repos only.
  - §9: where the label sits in each app.
- **DESIGN.md** :46-48: the automatic scan sentence.
- **TECHNICAL.md**: the `Channel` / `FolderScan` bridges in Scanning, and
  the `macos/ffi` line of the architecture tree.
- **ROADMAP.md**: tick the overlap item (:172-173); fix the stale :30-32
  (post-action rescans are not full scans); add the done items. The 20 s
  fetch item (:174-175) stays open.
- **README.md**: no change.

## Out of scope

- Persisting results across launches.
- Backoff, debounce or a separate cadence for failing remotes (ryujinx keeps
  its 21 s; it is now one spinning row instead of a frozen folder).
- Logging the trigger of each scan.
- Scan cancellation and the two apps scanning the same folder at once (two
  processes, two registries).

## Tests

- Core: done (`registry.rs`, `tests/scan_snapshots.rs`). Dropped: "two
  folders, one blocked, complete independently". The old bug needs the pool
  saturated (more than 32 repos that never answer), too slow and timing-bound
  for `just test`.
- `just clippy` (pedantic) clean for core, ffi and src-tauri.

## Slices for implementation (Code Mentor style)

In user-flow order, both apps in each slice from step 7 on:

1. ~~Core types and registry (steps 1–2), with their tests.~~ Done.
2. ~~Core streaming scan, recheck and pull's `record_fetch` (steps 3–4).~~
   Done.
3. Bridges (steps 5–6).
4. Coordinator: streaming, spinners, Checking section, joining (step 7
   minus rechecks).
5. Post-action rechecks (rest of step 7).
6. "Last scan" label and ticking (step 8).
7. Scheduler (step 9).
8. Docs (step 10).

## Manual test script

1. Launch with `Dev` and `Documents`. Expect every repo under *Checking* with
   spinners within a second, repos moving into sections one by one,
   `Documents` done in about a second, and ryujinx the last spinner in `Dev`.
   "Last scan: just now" appears next to the scan button.
2. Press Scan All. Expect every repo to stay in its section with a spinner
   (no *Checking* section this time), and the `Dev` header and sidebar row to
   keep showing their counts next to a spinner, the counts changing as repos
   move.
3. Switch to another app and back within a few minutes. Expect no scan.
4. Wait 10 minutes with the app open (focused or not). Expect no indicator
   while the scan runs, and changes from another app (e.g. a new uncommitted
   file) to appear on their own. The label returns to "just now".
5. During that silent scan, press Scan All. Expect indicators and spinners
   for what is still pending, and no second `scan started` line per folder in
   the log.
6. Pull a repo that is behind. Expect only that row to spin and move to its
   new section, without a spinner on its folder.
7. Watch the label across an hour: once at 60 s, then each minute, then
   hourly; refocusing updates it at once. Hover it for the absolute time.
8. Put the Mac to sleep for 15+ minutes, wake it and focus the app. Expect a
   visible full scan.
9. Repeat 1–8 in the Tauri app.
10. macOS with two windows open: one activation causes at most one scan.
