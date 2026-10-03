# Plan: streamed scans, a "Last scan" indicator and a calmer schedule

Status: implemented in both apps (core, bridges, coordinator, rechecks, "Last
scan" label, scheduler). FRONTEND.md §3, §5.1–5.3, §5.5–5.6 and §9, TECHNICAL.md
(Scanning) and DESIGN.md describe the result; the "why" is in
`docs/analysis/automatic-scan-strategy.md`. Delete this file once the manual
test script below passes.

## Out of scope

- Persisting results across launches.
- Backoff, debounce or a separate cadence for failing remotes (ryujinx keeps
  its 21 s; it is now one spinning row instead of a frozen folder).
- Logging the trigger of each scan.
- Scan cancellation and the two apps scanning the same folder at once (two
  processes, two registries).
- Keeping the Tauri app's WebView awake while minimized on macOS
  (`backgroundThrottling: "disabled"`): the focus rescan covers the return.

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
8. Put the Mac to sleep for 15+ minutes and wake it. Expect a silent scan
   about 30 s after wake (30–90 s in Tauri; the label returns to "just now"),
   or a visible one if you focus the app before that.
9. Edit a folder's "Only local checks" during a silent scan, then press Scan
   Folder. Expect the log to show the old scan finishing before a new
   `scan started` line for that folder.
10. Repeat 1–9 in the Tauri app.
11. macOS with two windows open: one activation causes at most one scan.
    Close every window and wait 10+ minutes: the log still shows a scan.
