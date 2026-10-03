# FRONTEND.md — Frontend Behavior Specification

Source of truth for how **both** frontends behave:

- **Tauri app** (`desktop/`, React + Tailwind) — Windows & Linux (still builds on macOS).
- **SwiftUI app** (`macos/`) — native macOS 26+.

Both sit on the same Rust core (`core/`, crate `gpm-core`) and share the same on-disk
stores, so a machine with both apps installed sees identical data.

**Scope**: Folders, Scanning, Settings, the Kanban board (§7), Account/sign-in
(§6.3), and the diagnostics log (§6.4) — all present in **both** apps.

This spec defines *behavior*. Both apps share the same layout — sidebar navigation,
an All Folders overview with actionable repos expanded inline, a per-folder detail
view, and the kanban board — but visual presentation follows each platform's idiom
(§9): the Tauri app keeps its dense dark developer-tool look; the macOS app uses the
native macOS 26 design language (Liquid Glass, system light/dark appearance) and is
*not* a pixel copy. The kanban design language is defined by the macOS app (tinted
column headers with count capsules, soft rounded surfaces, named relative dates,
sync status chip); the Tauri board mirrors it in its dark palette.

---

## 1. Domain model

```
MonitoredFolder {
  id: UUID-string, path: string, name: string,
  onlyLocalChecks: bool, detectUninitialized: bool
}

RepoStatus {
  path: string           // absolute repo path
  branch: string?        // current branch; absent for uninitialized/unborn
  hasChanges: bool?      // uncommitted changes (untracked included); nil = unknown
  hasUnpushed: bool?     // local commits ahead of upstream; nil = unknown/skipped
  hasUnpulled: bool?     // upstream commits not local; nil = unknown/skipped
  remoteStateUnknown: bool  // the comparison was attempted and failed
  publishState: enum     // "published" | "unpublished" | "remoteNotFound"
  hasError: bool
  errorMessage: string?
}

ScanResult {
  scannedPath, totalRepositories, executionTime (seconds, float; 0 until complete),
  startedAtMs (unix ms), revision (increases on every change), isComplete,
  pending[] (paths not checked yet), checking[] (found, no status yet),
  withChanges[], withUnpushed[], withUnpulled[], unpublished[], remoteNotFound[],
  remoteStateUnknown[], clean[], errors[], uninitialized[]
}
```

- The backend categorizes; frontends never re-derive categories from the flags.
  A repo may appear in several category arrays (e.g. changed *and* unpushed).
- There are three **overlay** categories — a repo in one *also* appears in its
  primary status bucket (e.g. a no-remote repo with edits is in both
  `withChanges` and `unpublished`). Errored and uninitialized entries are never
  included in an overlay. `publishState` drives the first two, which are
  mutually exclusive; `remoteStateUnknown` is independent of both.
  - `unpublished` = **no remote configured** (never pushed to a host).
  - `remoteNotFound` = a remote **is** configured but the host reports it is gone.
    Requires an online scan: `git fetch` must return "not found", `gh` must be
    signed in, **and** `gh` must confirm it. Anything uncertain (offline, auth
    failure, non-GitHub remote, no `gh`, or `onlyLocalChecks`) stays `published`
    — never a false positive. Confirmations are debounced (once per 24h per repo).
- `uninitialized` = directories that contain files but are not git repositories,
  found as siblings of discovered repos. Symlinked directories are skipped, the
  same way the repo walk skips them, so a link never produces a duplicate entry.
  Reported only for folders with `detectUninitialized = true`.
  - `remoteStateUnknown` = the unpushed/unpulled comparison **was attempted and
    failed**, so those counts are unknown rather than false. Without it such a
    repo fell through to `clean`, which asserted "nothing to push" about a check
    that never succeeded. Always empty for `onlyLocalChecks` folders — there the
    scan deliberately never asks, which is not a failure.
- `onlyLocalChecks = true` ⇒ scanner skips `git fetch` and unpushed/unpulled checks
  for every repo in that folder (fast, offline-safe); those fields come back nil.
  Because "no unpushed commits" then means "never asked", the UI must say so —
  see the indicator in §5.4.
- `detectUninitialized = false` ⇒ scanner skips the uninitialized walk for that
  folder and returns an empty `uninitialized` list; every other category is
  unaffected. The question "a project you forgot to `git init`?" only makes
  sense where every sub-folder is meant to be a project — asked of a
  general-purpose folder (Documents, say) it reports every ordinary directory
  and buries the repositories that are actually there.

## 2. Persistence contract (shared between apps)

All files live in `<config dir>/git-projects-manager/`
(macOS: `~/Library/Application Support/git-projects-manager/`). Pretty-printed JSON,
camelCase keys, written atomically (temp file + rename) by the core.

| File | Contents | Notes |
|---|---|---|
| `config.json` | `{ folders: [MonitoredFolder] }` | insertion order preserved; no sorting anywhere |
| `settings.json` | `{ defaultTerminal?, defaultEditor?, gitCleanSettings? }` | ids, not paths |
| `kanban_v2.json`, `repos_cache_v1.json` | kanban state / GitHub repo cache | shared by both apps (§7) |
| OS keychain (`git-projects-manager` / `sync_session`) | sync session | shared by both apps (macOS may prompt once before the other app may read the item) |
| `logs/macos.<date>.log`, `logs/desktop.<date>.log` | diagnostics log (§6.4) | one file per app per day, so two apps running at once never share a file |

Frontend-only state (scan results, expansion, search text, selected view) is **session
memory** — never persisted. Every launch starts fresh and rescans.

## 3. Application shell

| Behavior | Rule |
|---|---|
| Window | Single main window, resizable, ~1024×680 default, min ~800×540 |
| Startup | Start the diagnostics log (§6.4) first, then load folders + settings concurrently; failures degrade silently to empty state (and are logged) |
| Auto-scan | The first time the folder list becomes non-empty in a session, scan all folders once |
| Search | One search field filtering repo lists live (§5.4); session-only value |
| Scan | One primary toolbar action, targeting the current view: **Scan All** in the overview, **Scan Folder** in a folder's detail view (§5.1); disabled when no folders exist or its target is already scanning |
| Settings access | Tauri: sidebar gear button → Settings (modal). macOS: standard Settings scene (⌘,) plus folder management in the main window (§9) |

## 4. Folder management (CRUD)

- **Fields**: absolute path (free text + native directory picker), display name,
  and two toggles:
  - **"Only code projects"** (default **on**) — "Reports sub-folders without a
    git repository as Uninitialized." Off for folders that hold ordinary
    documents, where that finding is noise.
  - **"Only local checks"** (default off) — skips remote fetch and push/pull
    checks for faster, offline-friendly scans.

  A folder stored before a toggle existed keeps the behavior it had: a missing
  `detectUninitialized` reads as **true**.
- **Validation**: path and name must be non-empty after trimming — error
  "Path and name are required". The core additionally rejects a path that
  **overlaps** an already-monitored folder — the same directory, one inside the
  other, or one containing the other — because a repository under both would be
  scanned twice in the same pass, running two `git fetch` processes inside one
  `.git`. The message names the conflicting folder ("This folder is inside
  "Dev" (/Users/leo/Dev), which is already monitored…"). Editing a folder
  without moving it is never reported against itself. No existence check;
  values are otherwise stored as entered.
- **Add** appends to the end of the list. **Edit** mutates in place (position kept);
  unknown id errors "Folder not found". **Delete** is immediate, no confirmation —
  it only stops monitoring; nothing on disk is touched. Only one folder is editable
  at a time; switching targets silently discards unsaved edits.
- During any mutation all mutating controls disable; the submit label indicates
  progress ("Add…"/"Save…"). Failures show the reason reported by the core, so a
  rejected path explains itself; a generic banner ("Failed to add folder",
  "Failed to update folder", "Failed to delete folder") covers errors that carry
  no message. Details also go to the diagnostics log (§6.4).
- After every successful mutation the folder list is re-fetched and all views update.
  Re-fetching also drops the results and scan-progress state of any folder no
  longer in the list (§5.2).
- Empty state: "No folders configured yet." / main view: "No folders configured.
  Add a folder to get started."

## 5. Scanning

### 5.1 Scan modes

All modes scan every target folder **concurrently**, and each folder's state streams
onto the screen as the scan goes: once the walk has found its repos, then each time a
repo's status lands (§5.3). A folder whose scan fails keeps what it had; the failure is
shown nowhere but the diagnostics log (§6.4).

There is **one** scan control, in the toolbar/header, and what it scans follows
the view: the All Folders overview scans every folder, a folder's detail view
scans that folder. "Scan" always means "scan what I am looking at", so the two
modes below are the same button and the same keyboard shortcut (macOS: ⌘R), not
two competing controls. Per-folder Scan buttons still sit in each overview row
and in the not-scanned empty state, where they name their own target.

1. **Full scan** — the scan control in the overview, and the startup auto-scan.
   Shows global + per-folder progress.
2. **Per-folder scan** — the scan control in a folder's detail view, the
   per-folder buttons in the overview, and the automatic rescan after a
   pull/clean (scoped to the folders the affected repos live in, since an action
   on one repo cannot change another folder's state). Per-folder progress only.
3. **Focus rescan** — when the app window regains focus (after the initial scan,
   folders exist): rescan all folders as a full scan, so it shows the **same
   global + per-folder progress** as Scan All. Throttled to at most once per
   **20 seconds** since the last scan of any kind, and skipped while any scan is
   in flight.

Every scan still fetches — ahead/behind counts are meant to be current — but the
core skips the round-trip for any repo it fetched successfully in the last
**30 seconds**, reusing the tracking refs from that fetch. Scans arrive in
bursts (the rescan after a pull or clean, a focus rescan landing on the heels of
the startup scan) that repeat the same network work for a state that cannot have
changed; a repeat scan inside the window measures **3.4 s → 1.0 s** over 76
repos. The window is short enough that any scan following real work is a fresh
one. A *failed* fetch is never debounced, so an unreachable remote is retried on
the next scan.

### 5.2 One scan per folder (concurrency rule)

A folder never has two scans at once. A request for a folder already scanning
joins the scan in flight instead of starting another: Scan All during a
per-folder scan starts only the other folders, and waits for all of them. Every
state the core sends carries a revision that grows with each change, and one no
newer than what is shown is dropped, so a late arrival never overwrites newer
state. There is no cancel, in the UI or in the core; the scan control is
disabled while its own target is scanning.

A folder deleted while it is scanning leaves no trace: nothing more of it is
stored, and its scan stops counting as in flight when it ends.

### 5.3 Results display

Navigation is a sidebar + detail split, the same in both apps:

- **Sidebar**: an **All Folders** entry and a **Kanban** entry (§7), then the
  monitored folders (stored order), each showing, when > 0, an attention
  badge (changed + unpushed + unpulled + errors; always unfiltered), with a
  spinner beside it while the folder scans. The Tauri app pins Add Folder +
  Settings at the bottom (§9).
- **All Folders overview** (default view) — per folder, a sticky header:
  folder name + monospace path; status area showing `"{total} repos"` plus a
  clean count badge (unfiltered, with a spinner beside them while the folder
  scans), "Scanning…" before the folder's first result, or "Not scanned"; a
  per-folder **Scan** control (disabled while that folder scans) and an
  open-detail control. Below the header all of the folder's sections render
  expanded inline (fixed order, Checking first, Clean last), so pending
  commit/pull work is visible without opening a folder. A folder with no visible sections shows
  "No repositories found" ("No matching repositories" while a search filters
  everything out).
- **Per-folder detail** — all sections, fixed order, empty sections hidden,
  plus a footer `"Completed in {executionTime, 2 decimals}s"` once the scan is
  complete:

  | Section | Color | Row actions |
  |---|---|---|
  | Checking | gray (muted rows) | open actions only; repos the scan found with no status yet, shown only while the folder scans |
  | Uncommitted Changes | yellow | open actions; Fetch & Pull visible but disabled |
  | Unpushed Commits | orange | open actions; Fetch & Pull |
  | Unpulled Commits | purple | open actions; Fetch & Pull; section bulk "Fetch & Pull All (n)" |
  | Unpublished | blue | open actions only (no remote, so no Fetch & Pull); overlay — same repos also appear above |
  | Remote Not Found | pink | open actions only (remote is gone, so no Fetch & Pull); overlay — same repos also appear above |
  | Unknown Remote State | gray | open actions; Fetch & Pull; overlay — same repos also appear above |
  | Uninitialized | gray (muted rows) | open actions only |
  | Errors | red | open actions; Fetch & Pull visible but disabled; row shows errorMessage |
  | Clean | green (muted rows) | open actions; Fetch & Pull; Clean Ignored Files; section bulk "Clean All (n)" |

Within every section, repos are ordered alphabetically (case-insensitive, by
full absolute path — so they group by parent directory, A–Z within each group).
The core sorts once, so the order is identical in both apps and stable across
rescans.

Rows are a **single line**: a category color dot (softened to 50% opacity;
section headers carry the full-strength color), then the repo path in
monospace — directory muted, repo **name** emphasized — with the branch as an
accent-colored chip after it. When too narrow, the path truncates
**directory-first** (measured fit): the directory collapses to a trailing
`…/` bridge — never below a first-letter hint, so a nested repo can't be
mistaken for a root one — and the repo name middle-truncates only once the
hint plus the full name no longer fit; the full path becomes a tooltip.
Error rows add the errorMessage on a second line. A repo the folder's scan has
not checked yet shows a spinner in place of its action menu, as during a pull
or clean, and stays in its section until its new status moves it. Section headers show
`TITLE (filtered count)` in the category color, with a leading category dot
in the same column as the row dots.

### 5.4 Search semantics and the local-checks indicator

Case-insensitive substring match of the trimmed query against the repo **name**
(last path segment) **or full path**. Filters section contents and section counts;
sections filtered to zero disappear. Header badges and totals stay unfiltered.
**Bulk actions operate on the filtered list.** Folders themselves are never hidden.

A chip sits beside the scan control whenever the folders in view include any
with `onlyLocalChecks`. It reads **"Local checks only"** when every folder in
view is local-only, or **"{n} of {m} folders: local checks only"** when only some
are; it is absent otherwise. Scope follows the selection — the selected folder in
the detail view, all folders in the overview. Its tooltip names what is skipped
(fetch, unpushed and unpulled) and where to change it.

Without it, a local-only folder reports no unpushed and no unpulled commits for
every repo simply because it never asked, which on screen is indistinguishable
from genuinely being up to date. This is also what keeps the Unknown Remote
State section honest: that section means "we asked and failed", and the chip
covers the separate case of "we never asked".

### 5.5 Repo actions

Per-repo actions (context/row menu):

| Action | Availability | Behavior |
|---|---|---|
| Open in {editor} | default editor configured | opens repo in editor |
| Open in {terminal} | default terminal configured | opens repo dir in terminal |
| Open in LMS Github | always | runs `lms-github <path>` via login shell |
| Show in Finder | macOS only | reveals the repo directory in Finder (the Tauri app has no reveal action yet) |
| Copy Path | always | copies the repo's absolute path to the clipboard |
| Fetch & Pull | not for Uninitialized; disabled for Changed/Errors | `git pull --quiet` (which fetches); success → rescan of that repo's folder; failure → "Failed to pull {path}: {err}", where `{err}` is git's error alone (no progress lines); a pull still running after 5 minutes is killed and reported as timed out |
| Clean Ignored Files | Clean section only | `git clean -fdX` dry-run filtered by exclude patterns (§6.2), survivors deleted; 0 removed → "No ignored files to clean in {name}"; rescan of that repo's folder after |

Bulk **Fetch & Pull All** / **Clean All** run per-repo operations in parallel; if k
fail, report `"Failed to pull/clean {k} repo(s): {first failure}"` — the first
reason is included, since a bare count says neither which repo nor why. The
folders holding the affected repos are rescanned afterwards. In-flight repos show
a per-row spinner; bulk controls disable while running.

### 5.6 Error surfacing

One shared, non-dismissible error area shows the most recent scan/action failure.
An action sets its message **before** triggering its rescan, and that rescan does
not clear it — the message clears when the next *on-demand* scan starts (Scan All
or a per-folder Scan). Per-folder scan failures during a multi-folder pass are
silent (previous data kept).

The area shows at most two lines; hovering it shows the full text. Every message
it shows, and every message the kanban board shows, is also written to the
diagnostics log (§6.4), since the next message replaces it.

## 6. Settings

### 6.1 Default Apps

- Terminal/editor catalogs are compiled into the core (`core/resources/*.json`) and
  filtered to apps whose install path exists on disk.
- Selecting an app persists immediately (`defaultTerminal` / `defaultEditor` id);
  no Save button anywhere in Settings. Empty catalog → "No terminal applications
  found on your system." / "No code editors found on your system."
- No default configured ⇒ the corresponding "Open in …" action is unavailable.
- Open methods: editors via `open -a`; terminals via `open -a` or AppleScript
  (per-catalog `openMethod`), on macOS/Tauri alike.

### 6.2 Git Clean exclude patterns

- Ordered list of glob patterns preserved during Clean. Defaults when never saved:
  `.env*`, `*.key`, `*.pem`, `.vscode/`, `.idea/` (not written until first edit).
- Add: trims; rejects empty; exact-duplicate rejected with "Pattern already exists".
  Remove: per-row delete. Every change persists the whole array immediately.
- Matching rules: `*` and `?` wildcards only; trailing `/` = directory-component
  match (preserves the whole subtree); otherwise any path segment or the full
  relative path may match.
- Explainer text documents that Clean removes git-ignored files (`git clean -fdX`).

### 6.3 Account

Google sign-in via loopback PKCE; session in the OS keychain; exists solely to
sync the kanban board (§7). The explainer states the app works fully without
signing in. Signed-out: a "Sign in with Google" button ("Waiting for browser…"
while pending). Signed-in: shows `name || email || sub` (email as a second line
when both exist) and a Sign Out button. Errors render inline. Both the Settings
Account panel and the board's sync status chip menu (§7) offer sign-in/out.

### 6.4 Logs

A diagnostics log both apps write through the core, for troubleshooting failures
the UI shows briefly or not at all. It lives in the shared data folder (§2) under
`logs/`: one file per app per day (`macos.<date>.log`, `desktop.<date>.log`),
the newest **14** of each kept. The date in the name is UTC, the timestamps in
the lines are local time.

What it records:

- **Every git command** the core runs that fails, times out, or takes **5 s** or
  longer: the repo, the command, the duration, and git's full stderr on one line.
  Successful pulls and cleans are recorded too, and so is every path a clean
  could not delete, including the repos a bulk message leaves unnamed.
- **Every scan**: a start line and a finish line per folder, with the repo, error
  and unknown-remote counts. A start with no finish is a scan that hung.
- **Every repo check that failed** (the Errors bucket) and every repo whose
  ahead/behind could not be determined.
- **Every message the UI showed** (§5.6) and the failures it recovers from
  without a message: loading folders, settings or the kanban cache, folder scans
  (§5.1), saving settings, sign-in/out, the `gh` auth check, and opening the
  browser or the logs folder.
- The app version at startup, and any Rust panic with its backtrace. The Tauri
  app also records uncaught frontend errors (with their stack) and unhandled
  promise rejections.

Credentials embedded in URLs (`https://user:token@host`) are masked before a
line is written, since the log is meant to be shared when reporting a problem.

The **Logs** settings panel explains this, shows the folder's path (selectable),
and has an **Open Logs Folder** button that opens it in the system file manager;
a failure to open shows inline as "Failed to open the logs folder".

## 7. Kanban board

A sidebar view organizing the user's **GitHub repositories** as cards.

- **Data source**: `gh repo list` (GitHub CLI, limit 1000). Cards are never
  user-created: every repo appears exactly once, joined by `nameWithOwner`.
  On each refresh the board reconciles — new repos land in **Backlog**, cards
  for repos no longer on GitHub are pruned.
- **gh gate**: the board requires `gh` installed and authenticated. Otherwise
  a full-board empty state explains the fix (install gh / run `gh auth login`
  / the error message) with a **Recheck** action.
- **Columns** (fixed): Backlog (gray), Active · Low (blue), Active · High
  (red), Done (green), Closed (yellow). Unknown/legacy column ids display as
  Backlog. Column header: color dot + title + count capsule. Cards sort by
  `nameWithOwner` within a column.
- **Card**: repo name (+ `ARCHIVED` chip), owner login, lock glyph when
  private, relative pushed time in named form ("yesterday", "2 weeks ago");
  a third row with the card's notes when it has any (secondary color, small
  text, clamped to 3 lines, line breaks kept, in a rounded box), nothing
  otherwise, so cards without notes keep their height; tooltip shows the
  description. Card text is not selectable: a double-click opens the notes
  (see **Notes**).
- **Notes** — free text per card, edited in place. Clicking the notes text,
  double-clicking the card anywhere else, or the *Add Notes…* / *Edit
  Notes…* card action opens the editor; the last two are the entry points
  for a card without notes. The editor takes the box's place with an accent
  outline, focused with its text selected (typing replaces it; an arrow key
  or a click places the caret), placeholder "Add notes…", and grows with the
  text up to about 5 lines, then scrolls. Losing focus saves, by the save
  key (§9) or by a click anywhere outside the field, the card itself
  included; Escape restores the previous text. A double-click while editing
  only closes the editor; its second click does not reopen it. The
  saved text is trimmed, and an empty result clears the notes (the row
  disappears). Saving unchanged text is not an edit: nothing is written,
  `updatedAt` is not bumped, nothing syncs. The comparison is against the
  text the editor opened with, so a refresh that lands a remote edit under an
  open editor is not overwritten by an untouched draft. A card in edit mode
  cannot be dragged. Otherwise a save behaves like a move: optimistic,
  `updatedAt` bump, one-card background sync when signed in, refetch on
  failure. Notes travel with the card through cloud sync, whole card, so a
  move on one device and a notes edit on another resolve to the newer card.
  The save key differs per platform (§9).
- **Drag & drop** moves a card between columns (optimistic update; the store
  write bumps `updatedAt`; a one-card background sync runs when signed in;
  on failure the board refetches authoritative state). Dropping on the same
  column is a no-op; the target column highlights in its accent color.
- **Card actions** — hover ellipsis menu in both apps; macOS additionally
  offers the native right-click context menu: *Add Notes…* (no notes yet)
  or *Edit Notes…* (see **Notes**); *View on GitHub*; *Delete
  Repository…* — destructive, confirm dialog, offered only when the gh
  account owns the repo; runs `gh repo delete` then performs a full refresh
  (including cloud sync when signed in).
- **Refresh model**: first visit paints instantly from the offline cache
  (`repos_cache_v1.json`), then revalidates; a manual Refresh control; a
  window-focus revalidate debounced to 1.5 s; sign-in/out triggers a refresh.
  A full-board spinner appears only on first launch with no cache.
- **Cloud sync** (signed-in only): refresh pushes the full card set and
  merges the server's authoritative response (per-card last-writer-wins by
  `updatedAt`; local cards for repos the server hasn't seen are kept; server
  cards for repos gone from GitHub are dropped). Status per refresh:
  `disabled` (signed out) · `synced` · `offline` (network failure, local
  state kept) · `expired` (session rejected and cleared — sign in again).
  A **sync status chip** shows the status; its menu hosts sign-in/out (§6.3).
- **Board header**: most recent board error (left) and the authenticated
  `gh` account (right).
- **Search**: the macOS app's shared search field also filters cards and
  column counts (substring on `owner/name`); the Tauri app offers no kanban
  search (§9).

## 8. Non-goals / intentionally absent

- No scan-cancel UI, no folder reordering, no light theme in the Tauri app, no
  keyboard shortcuts in the Tauri app beyond native input behavior.
- No kanban card reordering within a column, no custom columns, no manual card
  creation (the GitHub repo list is the single source of cards). Card notes
  are not searchable: search matches `owner/name` only.
- Dead code from the web app (RadioGroup/SelectList) is not part of this spec and
  must not be ported.

## 9. Platform presentation mapping

| Concern | Tauri (Win/Linux) | SwiftUI (macOS 26+) |
|---|---|---|
| Chrome | Custom sidebar (§5.3) + content header (title, search, scan control); dark-only dense UI | `NavigationSplitView` sidebar (§5.3); Liquid Glass toolbar with the scan control. Registers a 350 ms `NSInitialToolTipDelay` so toolbar help text appears promptly and at the same speed everywhere |
| Appearance | Fixed dark palette | System light & dark, accent-aware; semantic colors for badge roles (green/yellow/orange/purple/blue/pink/gray/red) |
| Folder CRUD | Settings modal → "Monitored Folders" panel; sidebar **Add Folder** opens it | Main window: sidebar add button + sheet; edit via context menu/sheet |
| Settings | In-app modal via sidebar gear (Monitored Folders / Default Apps / Git Clean / Account / Logs) | Native Settings scene (⌘,): Default Apps, Git Clean, Account, Logs |
| Repo actions | Hover kebab dropdown (also on right-click) | Native context menu (right-click) + hover affordance |
| Search | Content-header text input (hidden on the kanban view) | `.searchable` toolbar field (also filters kanban) |
| Directory picker | Tauri dialog plugin | `NSOpenPanel` / SwiftUI fileImporter |
| Kanban (§7) | Sidebar item; HTML5 drag & drop; header row hosts sync chip + Refresh; notes save on Cmd/Ctrl+Enter (Enter adds a line) | Sidebar item; native drag & drop; toolbar hosts sync chip + Refresh; notes save on Return, Option+Return adds a line |
| Account (§6.3) | Settings modal panel + sync chip menu | Settings scene Account tab + sync chip menu |

Behavioral rules in §§1–8 are identical across platforms; only presentation differs.
