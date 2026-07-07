# FRONTEND.md — Frontend Behavior Specification

Source of truth for how **both** frontends behave:

- **Tauri app** (`desktop/`, React + Tailwind) — Windows & Linux (still builds on macOS).
- **SwiftUI app** (`macos/`) — native macOS 26+.

Both sit on the same Rust core (`core/`, crate `gpm-core`) and share the same on-disk
stores, so a machine with both apps installed sees identical data.

**Scope**: Folders, Scanning, Settings, the Kanban board (§7), and Account/sign-in
(§6.3) — all present in **both** apps.

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
MonitoredFolder { id: UUID-string, path: string, name: string, onlyLocalChecks: bool }

RepoStatus {
  path: string           // absolute repo path
  branch: string?        // current branch; absent for uninitialized/unborn
  hasChanges: bool?      // uncommitted changes (untracked included); nil = unknown
  hasUnpushed: bool?     // local commits ahead of upstream; nil = unknown/skipped
  hasUnpulled: bool?     // upstream commits not local; nil = unknown/skipped
  hasError: bool
  errorMessage: string?
}

ScanResult {
  scannedPath, totalRepositories, executionTime (seconds, float),
  withChanges[], withUnpushed[], withUnpulled[], clean[], errors[], uninitialized[]
}
```

- The backend categorizes; frontends never re-derive categories from the flags.
  A repo may appear in several category arrays (e.g. changed *and* unpushed).
- `uninitialized` = directories that contain files but are not git repositories,
  found as siblings of discovered repos.
- `onlyLocalChecks = true` ⇒ scanner skips `git fetch` and unpushed/unpulled checks
  for every repo in that folder (fast, offline-safe); those fields come back nil.

## 2. Persistence contract (shared between apps)

All files live in `<config dir>/git-projects-manager/`
(macOS: `~/Library/Application Support/git-projects-manager/`). Pretty-printed JSON,
camelCase keys, written atomically (temp file + rename) by the core.

| File | Contents | Notes |
|---|---|---|
| `config.json` | `{ folders: [MonitoredFolder] }` | insertion order preserved; no sorting anywhere |
| `settings.json` | `{ defaultTerminal?, defaultEditor?, gitCleanSettings? }` | ids, not paths |
| `kanban_v2.json`, `repos_cache_v1.json` | kanban state / GitHub repo cache | shared by both apps (§7) |
| OS keychain (`git-projects-manager` / `sync_session`) | sync session | shared by both apps |

Frontend-only state (scan results, expansion, search text, selected view) is **session
memory** — never persisted. Every launch starts fresh and rescans.

## 3. Application shell

| Behavior | Rule |
|---|---|
| Window | Single main window, resizable, ~1024×680 default, min ~800×540 |
| Startup | Load folders + settings concurrently; failures degrade silently to empty state |
| Auto-scan | The first time the folder list becomes non-empty in a session, scan all folders once |
| Search | One search field filtering repo lists live (§5.4); session-only value |
| Scan All | Primary toolbar action; disabled when no folders; shows in-progress state while a full scan runs; clicking again mid-scan supersedes the running scan (§5.2) |
| Settings access | Tauri: sidebar gear button → Settings (modal). macOS: standard Settings scene (⌘,) plus folder management in the main window (§8) |

## 4. Folder management (CRUD)

- **Fields**: absolute path (free text + native directory picker), display name,
  "Only local checks (skip remote fetch/push/pull checks)" toggle (default off).
- **Validation**: path and name must be non-empty after trimming — error
  "Path and name are required". No existence or duplicate checks. Values are stored
  as entered.
- **Add** appends to the end of the list. **Edit** mutates in place (position kept);
  unknown id errors "Folder not found". **Delete** is immediate, no confirmation —
  it only stops monitoring; nothing on disk is touched. Only one folder is editable
  at a time; switching targets silently discards unsaved edits.
- During any mutation all mutating controls disable; the submit label indicates
  progress ("Add…"/"Save…"). Failures show a generic banner ("Failed to add folder",
  "Failed to update folder", "Failed to delete folder"); details go to the log.
- After every successful mutation the folder list is re-fetched and all views update.
- Empty state: "No folders configured yet." / main view: "No folders configured.
  Add a folder to get started."

## 5. Scanning

### 5.1 Scan modes

All modes call the core scan once per target folder, **concurrently**; each folder's
result merges into the results map on completion. A folder whose scan fails keeps its
previous result silently.

1. **Full scan** — Scan All button, the startup auto-scan, and the automatic rescan
   after any pull/clean action. Shows global + per-folder progress.
2. **Per-folder scan** — per-folder Scan control; per-folder progress only.
3. **Silent scan** — when the app window regains focus (after the initial scan,
   folders exist): rescan all folders with **no UI indicators**, throttled to at most
   once per **20 seconds** since the last scan of any kind.

### 5.2 Supersession (concurrency rule)

Full scans carry a version. When a scan completes but a newer full scan started
meanwhile, its results are **discarded**, not merged. Per-folder scans are likewise
discarded if a full scan started after them. There is no user-facing cancel; the
core's cancellation API exists but is unused.

### 5.3 Results display

Navigation is a sidebar + detail split, the same in both apps:

- **Sidebar**: an **All Folders** entry and a **Kanban** entry (§7), then the
  monitored folders (stored order), each showing a scanning spinner or, when
  > 0, an attention badge (changed + unpushed + unpulled + errors; always
  unfiltered). The Tauri app pins Add Folder + Settings at the bottom (§9).
- **All Folders overview** (default view) — per folder, a sticky header:
  folder name + monospace path; status area showing "Scanning…", "Not
  scanned", or `"{total} repos"` plus a clean count badge (unfiltered); a
  per-folder **Scan** control (disabled while that folder scans) and an
  open-detail control. Below the header all of the folder's sections render
  expanded inline (fixed order, Clean last), so pending commit/pull work is
  visible without opening a folder. A folder with no visible sections shows
  "No repositories found" ("No matching repositories" while a search filters
  everything out).
- **Per-folder detail** — all six sections, fixed order, empty sections
  hidden, plus a footer `"Completed in {executionTime, 2 decimals}s"`:

  | Section | Color | Row actions |
  |---|---|---|
  | Uncommitted Changes | yellow | open actions; Fetch & Pull visible but disabled |
  | Unpushed Commits | orange | open actions; Fetch & Pull |
  | Unpulled Commits | purple | open actions; Fetch & Pull; section bulk "Fetch & Pull All (n)" |
  | Uninitialized | gray (muted rows) | open actions only |
  | Errors | red | open actions; Fetch & Pull visible but disabled; row shows errorMessage |
  | Clean | green (muted rows) | open actions; Fetch & Pull; Clean Ignored Files; section bulk "Clean All (n)" |

Rows are a **single line**: a category color dot (softened to 50% opacity;
section headers carry the full-strength color), then the repo path in
monospace — directory muted, repo **name** emphasized — with the branch as an
accent-colored chip after it. When too narrow, the path truncates
**directory-first** (measured fit): the directory collapses to a trailing
`…/` bridge — never below a first-letter hint, so a nested repo can't be
mistaken for a root one — and the repo name middle-truncates only once the
hint plus the full name no longer fit; the full path becomes a tooltip.
Error rows add the errorMessage on a second line. Section headers show
`TITLE (filtered count)` in the category color, with a leading category dot
in the same column as the row dots.

### 5.4 Search semantics

Case-insensitive substring match of the trimmed query against the repo **name**
(last path segment) **or full path**. Filters section contents and section counts;
sections filtered to zero disappear. Header badges and totals stay unfiltered.
**Bulk actions operate on the filtered list.** Folders themselves are never hidden.

### 5.5 Repo actions

Per-repo actions (context/row menu):

| Action | Availability | Behavior |
|---|---|---|
| Open in {editor} | default editor configured | opens repo in editor |
| Open in {terminal} | default terminal configured | opens repo dir in terminal |
| Open in LMS Github | always | runs `lms-github <path>` via login shell |
| Fetch & Pull | not for Uninitialized; disabled for Changed/Errors | `git fetch` + `git pull`; success → full rescan; failure → "Failed to pull {path}: {err}" |
| Clean Ignored Files | Clean section only | `git clean -fdX` dry-run filtered by exclude patterns (§6.2), survivors deleted; 0 removed → "No ignored files to clean in {name}"; always full rescan after |

Bulk **Fetch & Pull All** / **Clean All** run per-repo operations in parallel; if k
fail, report "Failed to pull/clean {k} repo(s)"; full rescan afterwards. In-flight
repos show a per-row spinner; bulk controls disable while running.

### 5.6 Error surfacing

One shared, non-dismissible error area shows the most recent scan/action failure;
it clears when the next on-demand scan starts. Per-folder scan failures during a
multi-folder pass are silent (previous data kept).

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
  tooltip shows the description.
- **Drag & drop** moves a card between columns (optimistic update; the store
  write bumps `updatedAt`; a one-card background sync runs when signed in;
  on failure the board refetches authoritative state). Dropping on the same
  column is a no-op; the target column highlights in its accent color.
- **Card actions** (hover menu + right-click): *View on GitHub*; *Delete
  Repository…* — destructive, confirm dialog, offered only when the gh
  account owns the repo; runs `gh repo delete` then rebuilds the board.
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

- No scan-cancel UI, no repo sorting, no folder reordering, no light theme in the
  Tauri app, no keyboard shortcuts in the Tauri app beyond native input behavior.
- No kanban card reordering within a column, no custom columns, no manual card
  creation (the GitHub repo list is the single source of cards).
- Dead code from the web app (RadioGroup/SelectList) is not part of this spec and
  must not be ported.

## 9. Platform presentation mapping

| Concern | Tauri (Win/Linux) | SwiftUI (macOS 26+) |
|---|---|---|
| Chrome | Custom sidebar (§5.3) + content header (title, search, Scan All); dark-only dense UI | `NavigationSplitView` sidebar (§5.3); Liquid Glass toolbar with Scan All |
| Appearance | Fixed dark palette | System light & dark, accent-aware; semantic colors for badge roles (green/yellow/orange/purple/gray/red) |
| Folder CRUD | Settings modal → "Monitored Folders" panel; sidebar **Add Folder** opens it | Main window: sidebar add button + sheet; edit via context menu/sheet |
| Settings | In-app modal via sidebar gear (Monitored Folders / Default Apps / Git Clean / Account) | Native Settings scene (⌘,): Default Apps, Git Clean, Account |
| Repo actions | Hover kebab dropdown (also on right-click) | Native context menu (right-click) + hover affordance |
| Search | Content-header text input (hidden on the kanban view) | `.searchable` toolbar field (also filters kanban) |
| Directory picker | Tauri dialog plugin | `NSOpenPanel` / SwiftUI fileImporter |
| Kanban (§7) | Sidebar item; HTML5 drag & drop; header row hosts sync chip + Refresh | Sidebar item; native drag & drop; toolbar hosts sync chip + Refresh |
| Account (§6.3) | Settings modal panel + sync chip menu | Settings scene Account tab + sync chip menu |

Behavioral rules in §§1–8 are identical across platforms; only presentation differs.
