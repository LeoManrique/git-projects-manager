# Plan: streamed scans, a "Last scan" indicator and a calmer schedule

Status: slices 1–6 of 8 done (core, bridges, coordinator, rechecks, "Last
scan" label). Next: slice 7, the scheduler. The "why" is in `docs/analysis/automatic-scan-strategy.md`. Delete this file once the
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
  hidden; a new repo appears once checked.
- **Joining.** Pressing Scan All during a silent scan does not start a second
  one: the folders already scanning switch to visible (indicators and
  spinners appear for the repos still pending), and only idle folders start.
- **Focus.** Returning to the app scans only if the last full scan started
  15+ min ago. With the timer at 10 min this only happens when the OS held the
  timer back (App Nap, sleep, a throttled WebView), which is exactly when the
  screen may be stale.

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

### 1–8. Core, bridges, coordinator, rechecks and "Last scan" (done)

TECHNICAL.md (Scanning) and FRONTEND.md §5.1–5.3 and §5.5 describe them. What
the next slices need:

- **Coordinator**: `requestScan(targets, visible)` in `AppModel` /
  `useScanner` is the one entry point; every caller passes `visible: true`
  so far. Snapshots go through `apply(_:to:)` (macOS) / `applySnapshots`
  (Tauri), which drop a gone folder or an older revision.
- **Clock** `lastFullScanStartedAt` (`AppModel` / `useScanner`): set by
  `scanAll` / `fullScan` when the request *ends*, from the starts
  `requestScan` now returns (min across folders, joined ones included, never
  backwards). Set at the end rather than on the first snapshot, so the label
  never claims a refresh still running; the timer below re-arms after a run
  settles anyway. Step 9's silent full request must record it the same way.
- Rechecks never touch the in-flight map or the clock: a recheck is not a
  scan, so step 9's rules can ignore them.
- Dropped: skipping a snapshot equal to the one shown. Every snapshot has a
  new revision, and re-rendering an unchanged screen is cheap.
- Found on the way: with more queued repos than pool threads, a folder's jobs
  still queue behind another folder's; they wait for those to *start*, no
  longer to finish.

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
- **Joining after an edit**: a join matches by folder id, so once silent scans
  exist, Scan Folder right after editing a folder joins a scan still running
  with its old path or settings. Wait for that scan, then start a new one.
- **No window (macOS)**: the timer keeps running in `AppModel` only while a
  window exists; with none, it parks until one opens.

### 10. Docs

Streaming, joining, the slice-4 display, rechecks and the label are already
in FRONTEND.md §2, §5.1–5.3, §5.5 and §9, and ROADMAP.md.

- **FRONTEND.md**
  - §3: auto-scan row, scheduler rows.
  - §5.1: triggers rewritten (launch, timer, focus 15 min, manual), visible vs
    silent (a silent scan joined by a visible request turns visible).
- **DESIGN.md** :46-48: the automatic scan sentence.
- **ROADMAP.md**: add the done items. The 20 s fetch item stays open.
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
3. ~~Bridges (steps 5–6).~~ Done.
4. ~~Coordinator: streaming, spinners, Checking section, joining (step 7
   minus rechecks).~~ Done.
5. ~~Post-action rechecks (rest of step 7).~~ Done.
6. ~~"Last scan" label and ticking (step 8).~~ Done.
7. Scheduler (step 9).
8. Docs (step 10).

## Manual test script

1. Launch with `Dev` and `Documents`. Expect every repo under *Checking* with
   spinners within a second, repos moving into sections one by one,
   `Documents` done in about a second, and ryujinx the last spinner in `Dev`.
   "Last scan: just now" appears next to the scan button once it ends.
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
