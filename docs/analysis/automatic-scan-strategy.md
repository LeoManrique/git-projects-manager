# Automatic scan strategy: analysis

Date: 2026-10-03. Scope: why both apps seem to be "Scanning…" most of the
time, what the scan triggers do today, what the logs measure, how leogit keeps
its repos fresh, and which decisions a less aggressive strategy with a "Last
Scan" indicator and background scanning would raise. This is analysis only. It
picks no option.

Sources:

- the macOS app (`macos/`, `macos/ffi/`), the Tauri app (`desktop/`) and the
  core (`core/`) at commit `222c43b`
- the diagnostics logs `macos.2026-09-25.log` … `macos.2026-10-03.log`: 176
  lines, 6 app sessions, macOS app only (there are no Tauri logs)
- leogit (`~/Dev/LeoManrique/Desktop/leogit`), both of its frontends

## 1. Summary

The "always scanning" feeling comes from several things adding up:

1. **One unreachable repo sets the length of every `Dev` scan.**
   `Dev/Others/ryubing/ryujinx` points at `git.ryujinx.app`, which does not
   answer. Its fetch times out at 20 s in 20 of 22 `Dev` scans and fails after
   13 s in the other 2. That is about 21 s of a median 22.3 s scan. No other
   repo's git call ever reached 5 s. Without it, a `Dev` scan is estimated at
   2–6 s.
2. **Failed fetches are never debounced.** The core's 30 s per-repo fetch
   debounce records only successful fetches. So a rescan right after a scan
   still pays the full 21 s for ryujinx: `Documents` drops to 0.11 s, while
   `Dev` stays at 22 s.
3. **On macOS, the focus throttle has always run out by the time a scan
   ends.** The 20 s count starts when a scan *starts*, and every `Dev` scan
   takes 20 s or more. So the first focus after any scan starts a new one
   immediately. The logs show a rescan starting 0.2 s after the previous one
   finished. The Tauri app counts from the scan's *end*, so the two apps
   behave differently under the same rule as FRONTEND.md states it.
4. **A focus rescan is a full scan with full progress UI.** It scans every
   folder and puts "Scanning…" in the toolbar, in every overview header and
   in every sidebar row. It was silent until commit `089764d` (2026-07-08).
   That commit does not record why it changed.
5. **"Scanning…" hides the numbers, even though the old results stay.** No
   result is cleared during a scan, and Pull and Clean stay enabled. But the
   overview header's repo count and clean badge, and the sidebar's attention
   badge, are replaced by a spinner. So the screen reads as "wait" even
   though most of what you need is already on it.
6. **Every session and every return starts with a scan.** Results live in
   memory only. A launch starts empty and has to wait for the slowest folder.
   In the logs, every period of activity (a cluster of lines with no gap over
   10 min) begins with a scan. Across those periods, a scan was running 34% of
   the time. In 7 of the 12 periods it was running 83–100% of the time.
7. **Nothing records when a scan happened.** `ScanResult` carries a duration
   (`execution_time`) but no timestamp. The only time state is the in-memory
   throttle clock, and neither app shows it.

## 2. What triggers a scan today

Both apps send every scan through one function: `AppModel.scan(folders:isFullScan:clearError:)`
in `macos/GitProjectsManager/Models/AppModel.swift`, and `scan()` in
`desktop/src/hooks/useScanner.ts`. Each folder is one `scan_folder` call into
the core. The calls for several folders run concurrently. Each returns one
complete `ScanResult`, with no streaming per repo.

| Trigger | Folders | Guard | Progress shown |
|---|---|---|---|
| Launch (first non-empty folder list) | all | once per session | global + per folder |
| App or window focus | all | 20 s throttle; skipped if any scan is in flight | global + per folder |
| Toolbar Scan All / Scan Folder, ⌘R (macOS only) | all, or the folder on screen | button disabled while that target is scanning | global, or per folder |
| Per-folder scan controls (header, context menus, empty states) | one | disabled while that folder is scanning | per folder |
| After a single pull or clean (success only) | the folder holding the repo | **none** | per folder |
| After Pull All or Clean All (always) | the affected folders | **none** | per folder |

What does **not** start a scan:

- adding a folder after the first one (it stays "Not scanned")
- editing a folder, even toggling *Local checks only* or *Only code projects*
  (the old result stays)
- deleting a folder
- settings changes
- opening a repo in a terminal, editor or Finder. Switching apps does mean
  that coming back fires the focus trigger.

There are no timers in either app.

### Where the two apps differ

| | macOS | Tauri |
|---|---|---|
| Focus signal | `NSApplication.didBecomeActiveNotification` (app activation) | DOM `window` `focus` event, which probably also fires when the folder picker closes and on DevTools switches (not verified) |
| Throttle clock | `lastScanStartedAt`, stamped when **any** scan starts | `lastScanTimeRef`, stamped when **any** scan finishes, including superseded ones |
| Quiet time after a 22 s scan ends | none | 20 s |
| Listener lives on | each window's view: two windows double-scan, and no window means no focus rescans | the single webview |

FRONTEND.md §5.1 says "at most once per 20 seconds since the last scan of any
kind". It does not say whether that counts from the start or the end of a
scan, which is how the apps came to differ.

### Known gaps in the trigger code

- **Post-action rescans ignore an in-flight scan of the same folder.** This is
  already a ROADMAP item. On 09-27 there were three concurrent `Dev` scans: a
  focus scan, a post-pull rescan and a post-Clean All rescan. All three ended
  at the same instant, 21.8, 42.7 and 50.2 s after their starts.
- **Superseded scans keep running.** Supersession discards a scan's result,
  but nothing can cancel it in the core, so all of its fetches still run.
- **The focus guard is all-or-nothing.** Any per-folder scan in flight blocks
  every focus rescan.
- **macOS, two windows:** one activation calls the focus handler once per
  window in the same run-loop turn. Both calls pass the guards before either
  scan sets its flags.
- **The trigger is not logged.** `scan started` records only the folder and
  `only_local_checks`, so the logs cannot tell a focus scan from a ⌘R.

## 3. What the logs measure

There are 42 `scan started` lines: 24 for `Dev` (62–63 repos) and 18 for
`Documents` (3 repos), in 24 scan events. Every scan was online. No folder
uses *Local checks only*.

| | Dev | Documents |
|---|---|---|
| Finished scans | 22 (2 never finished; the app was likely quit mid-scan) | 18 |
| Duration min / median / p90 / max | 14.2 / 22.3 / 33.4 / 50.2 s | 0.11 / 1.59 / 21.5 / 22.5 s |
| Starts under 60 s apart | 9 of 23 intervals | 3 of 17 |
| Rescan started within 21 s of the previous scan ending | 7 times (0.2, 4.7, 4.8, 7.7, 13.3, 17.3, 20.6 s) | — |

Inferred triggers (only inferred, since the logs do not record them):

- 6 launch scans
- 6 post-action rescans
- 12 focus or manual scans: 8 after idle gaps of 3 min to 25 h, and 4 rapid
  repeats

### The ryujinx fetch

- It is in every finished `Dev` scan: 20 timeouts at 20.0–21.1 s, and 2
  connection failures at 13.3 s.
- In 20 of 22 scans, `scan finished` follows the ryujinx line within 2–8 ms.
- It starts a median 1.25 s into the scan, so about 94% of a `Dev` scan is
  spent waiting on this one repo.
- A failed fetch is not recorded in the debounce map, so it is retried on
  every scan. A `RemoteNotFound` check never helps here: the error is
  "couldn't connect", not "not found".
- The ROADMAP already lists the 20 s limit as a separate problem: a repo that
  needs more than 20 s never completes a fetch, because the killed fetch
  keeps nothing.

### Folders are not independent

- In 3 full scans, `Documents` finished at the same microsecond as `Dev`,
  after about 22 s instead of about 1 s.
- All folder scans share one rayon `SCAN_POOL`. A worker waiting inside one
  folder's scan can pick up another folder's ryujinx job. This explanation is
  inferred from the timestamps.
- So one stuck repo can keep the spinner on folders that do not contain it.

### Cost of the local part

Two figures from earlier measurements in `docs/SCANNER_PERFORMANCE.md`:

- A local-only scan of ~70 repos takes about 0.46 s. This uses libgit2, with
  no network.
- A repeat online scan of 76 repos, inside the fetch debounce, takes about
  1.0 s.

The network fetch is almost the whole cost of an automatic scan, and its
worst case is set by the slowest remote, not by the number of repos.

## 4. What goes stale, and how fast

The sections a scan fills come from two kinds of source.

| Section | Source | What changes it |
|---|---|---|
| Uncommitted Changes, Unpushed, Unpublished, Uninitialized | local disk only | the user, in another app (an editor, a terminal) |
| Unpulled, Remote Not Found, Unknown Remote State | a `git fetch` (and `gh` for Remote Not Found) | other people or machines pushing, or a remote going away |

- **Local state** changes mostly while the user is away from the app, and
  reading it is cheap (under 0.5 s for ~70 repos).
- **Remote state** changes on other people's schedules, needs the network,
  and is where the 20 s outliers come from.
- Today one trigger refreshes both together, so a focus rescan always pays
  for the network to refresh the local part too.

## 5. How leogit does it

leogit is a git client focused on one open repo, with a list of recent repos.
Its two frontends (SwiftUI and Tauri) use the same policy and numbers. The
shared core has no scheduler.

| Mechanism | leogit | git-projects-manager today |
|---|---|---|
| Local status of the open repo | silent poll every 2 s focused, 10 s unfocused, 30 s hidden | only as part of a full scan |
| Network fetch, open repo | silent auto-fetch every 30 s (configurable 5 s–1 h, on/off); ×3 when hidden; random 0–30 s skew on the first one | only as part of a scan |
| Other repos | three recency tiers (4 repos every 2 min, 5 every 5 min, 10 every 10 min), one fetch at a time, paused when the app is not focused, one catch-up on return | every repo of every folder, all at once, on each focus |
| Return to the app | a silent resync: fetch (60 s cooldown), status, top-tier sweep (at most once per 30 s) | a full scan of all folders with full progress UI |
| Duplicate work | per-repo in-flight sets; a sequential loop that never overlaps itself | none for post-action rescans |
| Failures and offline | a circuit breaker opens after 2 failures (30 s → 5 min backoff); offline, a fetch becomes a local recompute; no banners | failed fetches retried in full every scan |
| Filesystem watching | none, rejected on purpose: polling costs ~24 ms per tick, and a watcher "removes the scheduling, not the cost" | none |
| Progress shown for background work | **none**; old data stays, and unchanged results publish nothing | full global and per-folder "Scanning…" |
| "Last fetched" indicator | **not built**: an open ROADMAP item, though the in-memory timestamps exist | not built; no timestamp exists |
| Relative time format | shared `just now` / `N minutes ago` / …, absolute date in a tooltip, re-ticks every 10 s while visible | — |
| Power | macOS holds an App Nap assertion while a repo is open; the Tauri WebView may throttle hidden timers | — |

Rationale recorded in leogit's docs:

- "Stale in plain sight" is the failure to avoid.
- A hidden window keeps refreshing slowly so that coming back shows a current
  screen rather than a catch-up happening in front of the user. An earlier
  build paused everything when unfocused and went stale.
- Only the fan-out across many repos is "genuinely deferrable".
- Process spawns are the main performance lever.

leogit's `docs/plans/io-and-network-efficiency.md` names a sibling project's
`SCANNER_PERFORMANCE.md` as what prompted it, which is probably this repo.

### What does not carry over directly

- leogit refreshes about 20 repos in tiers based on how recently they were
  opened. This app has no idea of "recent". Every repo in a monitored folder
  counts the same, and a folder holds 60+ repos.
- leogit's background fetch has a 12 s hard limit. This app's is 20 s, and
  the long tail is one unreachable host, not slow transfers.
- leogit keeps one repo's status on screen. This app shows a categorized
  overview, where a repo moving between sections is the information itself.

## 6. Decisions this raises

These are the questions a change would have to answer. For each, the facts
above that bear on it. None is answered here.

### 6.1 When to scan automatically

- **Which triggers stay:** launch, focus, after an action, a timer, a network
  change.
- **What the focus throttle measures:** time since the last start, the last
  finish, or the age of each folder's last result.
- **How long the window is:** 20 s is shorter than one `Dev` scan.
- **Whether focus refreshes local state only** and leaves the network for
  less frequent runs. Local is ~0.5 s; the network is where the 21 s comes
  from.
- **Whether a short absence counts at all.** Switching to a terminal from a
  repo's row and back is the most common case.

### 6.2 What to scan

- All folders, the folders on screen, or only folders whose results are older
  than some age.
- Every repo the same, or slow and failing repos handled separately: fewer
  attempts, a backoff, or a debounce for failures too.
- How to keep one slow repo from holding other folders' results, given the
  shared `SCAN_POOL`.

### 6.3 What the user sees while a scan runs

- Whether an automatic scan shows the global indicator, a per-folder one,
  something quieter, or nothing. leogit shows nothing. This app showed nothing
  until July.
- Whether "Scanning…" should keep replacing the repo counts and badges, given
  the data under them stays valid.
- Whether a manual scan should look different from an automatic one.

### 6.4 The "Last Scan" indicator

- **Which timestamp:**
  - the scan's start, since the fetch reflects the remote around then;
  - its finish, which is what the user sees;
  - or two separate times, one for local and one for remote, if those get
    split.
- **Per folder, global, or both:**
  - All Folders has one header per folder.
  - A folder's detail view already shows "Completed in Xs".
  - The sidebar has room for a badge only.
- **Format:**
  - relative ("2 min ago"), with or without an absolute tooltip;
  - how often it re-ticks;
  - whether to reuse leogit's vocabulary.
- **Whether failures count:**
  - a scan whose fetches failed ("scanned, but N repos could not reach their
    remote");
  - a folder whose last scan failed and kept older results.
- **Whether the core provides the timestamp in `ScanResult`.** That would
  serve both apps and keep the time with the result. Otherwise each app
  stamps its own.

### 6.5 Persisting results across launches

- Today every launch shows nothing until each folder finishes.
- Keeping the last results on disk with their timestamp would let a launch
  show the previous state, marked with its age, right away.
- Questions:
  - The format, and whether it is versioned.
  - Which app writes it. Both apps share the data directory, and the core
    already does atomic, per-process-unique writes.
  - How a result for a folder that was since edited or removed is dropped.
- The FRONTEND.md §2 statement "never persisted" would change.

### 6.6 Background scanning

- **What "background" means:**
  1. scanning while the window is open but not focused;
  2. while it is hidden or minimized;
  3. while no window is open (macOS keeps the app alive; Tauri quits on its
     last window, and has no tray).
- **Platform limits:**
  - macOS App Nap throttles timers in a hidden app unless an activity
    assertion is held. leogit holds one.
  - The Tauri WebView may throttle hidden timers.
- **Cadence:**
  - fixed or configurable;
  - per folder (each folder already has a *Local checks only* setting);
  - whether it backs off on failure, offline or battery, none of which leogit
    handles.
- **Results:**
  - Whether a background result replaces the screen silently or marks what
    changed.
  - The overview's whole value is which section a repo is in, so a silent
    move could go unnoticed.
- **Overlap:** whether background, focus and post-action scans share one
  queue per folder. Today nothing prevents two scans of one folder.

### 6.7 Diagnostics

- Whether to log the trigger of each scan (launch / focus / manual /
  post-action / timer), so the effect of any change can be measured.
- Whether successful fetches should log their duration at INFO. Today they
  are DEBUG, so the logs cannot show the per-repo spread.

## 7. What could not be determined

- How many focus events the throttle or the in-flight guard skipped. Neither
  is logged.
- How often the user actually waited on a scan, as opposed to it running
  unseen. There are no focus, blur or quit lines.
- Per-repo fetch times for successful fetches, which are logged at DEBUG.
- Tauri behaviour in practice. There are no `desktop.*.log` files.
- Why the focus rescan stopped being silent in `089764d`.
