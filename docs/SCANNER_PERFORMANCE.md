# Scanner Performance & Reliability Audit

Why scans were slow, why results were inconsistent between runs, and where I/O
and network work can still be removed. Kanban is out of scope.

**Status: Tier 1 is implemented** (except #1, which needs a product decision —
see §6 Q1). `DESIGN.md`, `FRONTEND.md` and `TECHNICAL.md` describe the behavior
as it is now; this document keeps the reasoning and the remaining work.

Measure before and after with `just bench-scan <path> [local]`, which prints
three timed runs plus every bucket count — so a change can be shown to be faster
*and* to still find the same repos.

---

## 1. Measured results

`/Users/leo/Dev`, 76 repos, warm cache, 12 logical CPUs, all-HTTPS remotes:

| | Before | After |
|---|---|---|
| Online scan | 4.7 s | **2.3 s** |
| Local-only scan | — | 0.5 s |
| Bucket counts | — | identical |

The remaining 2.3 s is almost entirely 76 `git fetch` round-trips. On a slow or
partly-unreachable network the gap is far wider than 2×, because the old code
had no bound on how long a single fetch could hold a worker.

Two costs measured but **not yet removed** (both blocked on §6 Q1):

- Directory walk: **1.01 s → 0.05 s** if the walk stops at a repo root.
  13,536 of 16,645 visited directories (**81%**) are inside a repo already found.
- 7 nested repos would stop being listed.

---

## 2. What changed

### 2.1 Correctness

| Fix | Where |
|---|---|
| A `gh` missing from `PATH` no longer reads as "repository deleted" | [github_cli.rs](../core/src/infrastructure/github_cli.rs) |
| `git clean` handles non-ASCII paths | [git.rs](../core/src/infrastructure/git.rs) |
| One unremovable path no longer aborts a repo's whole clean | [git.rs](../core/src/infrastructure/git.rs) |
| `git remote` / `git log` failures no longer return a confident wrong answer | [git.rs](../core/src/infrastructure/git.rs) |
| Repo ordering is a total order | [scanner.rs](../core/src/domain/scanner.rs) |
| Bulk actions report the first failure reason, not just a count | both frontends |
| Tauri finally shows bulk action messages at all | [useScanner.ts](../desktop/src/hooks/useScanner.ts) |
| macOS clears a folder's spinner only if the scan still owns it | [AppModel.swift](../macos/GitProjectsManager/Models/AppModel.swift) |

**The `gh` classifier.** `gh` runs through `$SHELL -lc`. When `gh` is not on the
login `PATH` the *shell* prints `zsh:1: command not found: gh` — which contained
`"not found"`, so the classifier returned `NotFound`, the repo was promoted to
`RemoteNotFound`, and the verdict was cached for 24 h. `check_auth` had always
guarded this string; the second call site never did. That guard is now shared,
and the positive test is GitHub's actual wording rather than a bare `"not found"`
substring.

Because `$SHELL -lc` is a *login, non-interactive* shell it sources `.zshenv` /
`.zprofile` / `.zlogin` but **never `.zshrc`**, so this could fire from Finder
and not from a terminal — same repos, different badges. Verified **not currently
live on this machine**: `gh` resolves at `/opt/homebrew/bin/gh` in a bare login
shell, and `gh auth status` is healthy.

**`git clean` and non-ASCII paths.** Reproduced directly: with `core.quotePath`
at its default `true`, `git clean -fdXn` prints `Would remove "caf\303\251/"`.
`strip_prefix("Would remove ")` took that literally, `repo_path.join(…)` pointed
at a path that does not exist, and the `?` on the delete aborted the whole repo —
**the same repos failing on every run, reported only as a count**. This is the
"Failed to clean 2 repo(s)" banner. Two changes: `core.quotePath=false` on every
git invocation, and per-path error collection (an already-gone path counts as
success; real failures are collected and named). Covered by
[core/tests/clean_paths.rs](../core/tests/clean_paths.rs), which fails against
the old behavior.

Note the two fixes are **both** required: with error tolerance alone, a quoted
path would silently be skipped and the clean would report success having done
nothing.

### 2.2 Throughput

**Dedicated scan thread pool** ([scanner.rs](../core/src/domain/scanner.rs)) —
`4 × CPUs` clamped to 8–32, instead of rayon's global CPU-sized pool. Nearly all
of a status check's wall time is a fetch blocked on DNS/TLS, so a CPU-sized pool
turned 76 repos into 7 serialized waves. **This single change is the 4.7 s →
2.3 s.** Kept off the global pool so no other rayon user inherits a thread count
sized for blocking I/O.

**Fetch hardening** ([git.rs](../core/src/infrastructure/git.rs)) — `gc.auto=0` +
`maintenance.auto=false` (the default `--auto` maintenance forks a second
background repack process *per repo*, which is why one scan was occasionally and
inexplicably slower than the last), `--no-tags --no-recurse-submodules` (nothing
in a scan result uses either), `http.lowSpeedLimit`/`lowSpeedTime` and an SSH
`ConnectTimeout` + `BatchMode` (libcurl's default connect timeout is 300 s and
SSH has none, so one black-holed route could hold a worker for minutes).

**`pull` no longer pre-fetches** — `git pull` *is* fetch + merge, so the explicit
`fetch` before it was a second full network round-trip whose result was discarded.

**Scoped rescans after pull/clean** — an action on one repo cannot change another
folder's state, so only the folders holding the affected repos are rescanned.
Cleaning ignored files in one repo used to cost 76 network fetches. Both frontends
map repo → folder by longest path prefix, falling back to all folders when a repo
can't be attributed.

**`spawn_blocking` in the Tauri commands**
([commands/scan.rs](../desktop/src-tauri/src/commands/scan.rs)) — all four were
`async fn` with fully blocking bodies, so each occupied a tokio worker for its
whole run and one Clean All saturated the runtime. The macOS FFI bridge had
always done this correctly.

**Per-folder streaming in Tauri** ([useScanner.ts](../desktop/src/hooks/useScanner.ts))
— results were awaited as one `Promise.all` and merged in a single `setResults`,
so every folder appeared at the speed of the slowest one. Each folder now merges
as it lands, which is what `FRONTEND.md` §5.1 always specified and what the macOS
app always did.

### 2.3 Error surfacing

`setError(…)` in the Tauri app was immediately followed by `scan()`, whose first
synchronous statement was `setError('')` — set and cleared in the same React
batch, so **four messages never rendered at all**. On macOS the mirror-image bug:
the message was set *after* `await scanAll()`, so it appeared only once the rescan
finished and then outlived it.

Both now follow one rule, written into `FRONTEND.md` §5.6: an action sets its
message *before* triggering its rescan, and that rescan does not clear it. The
message clears on the next *on-demand* scan. The dead
`setError('Failed to scan folder(s)')` branch was removed — `performScan` caught
every rejection, so the `Promise.all` could never reject.

---

## 3. Corrections to the first draft of this audit

Recorded because they were asserted confidently and were wrong.

- **`git clean` has no `-z` flag.** The suggested `-z` + NUL splitting does not
  exist for this subcommand. `core.quotePath=false` is the whole fix.
- **`LC_ALL=C` is a no-op on this machine.** Apple Git ships no translations, so
  `"Would remove "` is English regardless of `LANG`. It is still set, because the
  Tauri app also targets Linux, where git *is* translated.
- **The five same-timestamp `exists: false` entries in `remote_checks_v1.json`
  were not evidence of mass false-flagging.** All five point at directories that
  no longer exist — those repos were genuinely deleted. What the file does show
  is that **the cache is never pruned**: 5 of its 6 entries are dead paths.
- **The `gh` bug is latent here, not active.** It is a real correctness bug and
  worth the three lines, but it is not the explanation for this machine's
  behavior. The likelier culprits were the unbounded fetches (§2.2) and the
  `clean` parser (§2.1).
- **`finder.rs` does prune.** It calls `skip_current_dir()` for `.git`, ~60
  excluded names and hidden dirs. What it does not do is stop at a repo root —
  which is the 81%, and which is a product decision, not an oversight.

---

## 4. Still open — highest value first

| # | Fix | Gain | Effort |
|---|---|---|---|
| 1 | One `Repository` handle per repo; replace 4 subprocesses with libgit2 | ~300 fewer process spawns + 76 fewer libgit2 opens per scan | M |
| 2 | Wall-clock timeout + kill on `git fetch` and `gh` | bounds the worst case for *all* transports | M |
| 3 | Gate `RemoteNotFound` on one `check_auth()` per scan | removes the §5.2 false-positive class | M |
| 4 | Fold uninitialized detection into the single directory walk | removes a second and third walk of the same tree | M |
| 5 | Share one `RemoteCheckCtx` across concurrent folder scans | stops lost updates; makes the 24 h debounce actually hold | M |
| 6 | Resolve the `gh` binary once; stop using a login shell | 100–500 ms of profile sourcing per call | M |
| 7 | Prune the remote-check cache of dead paths | unbounded growth; 5/6 entries are stale today | S |
| 8 | Two-phase scan: local results immediately, network results after | perceived latency → under a second | L |
| 9 | Take `git fetch` off the scan path (TTL / explicit refresh) | removes 76 network calls per scan | L |
| 10 | Distinguish "checked, false" from "could not check" in `RepoStatus` | stops unverifiable repos rendering as Clean | M–L |

### 4.1 Five subprocesses per repo where one libgit2 handle would do

The comment at `has_unpushed_commits` — *"Using git command for simplicity as
git2 branch tracking is complex"* — is the root of this class. Everything except
`fetch` and `clean` is a **local** operation git2 performs in-process:

| Current | Replacement |
|---|---|
| `git remote` | `repo.remotes()?.is_empty()` |
| `git rev-parse @{upstream}` | `repo.branch_upstream_name(…)` |
| `git log @{upstream}..HEAD` + `git log HEAD..@{upstream}` | one `repo.graph_ahead_behind(local, upstream)` |
| Two `Repository::open` per repo | one open, passed down |

Both `git log` calls run a full revwalk and format `--oneline` output that is
then thrown away — only `is_empty()` is read. Measured cost of the four local
subprocesses: ~10 ms each, ~3.1 s serialized across 76 repos (~0.3 s at current
parallelism). The bigger prize is that it makes #2 and #10 straightforward.

### 4.2 No wall-clock timeout

The fetch flags in §2.2 bound the two common transports, but there is still no
hard timeout on any subprocess. `std` has none built in: use `Child::try_wait()`
in a poll loop with `kill()` after ~10 s, or pull in `wait-timeout`. This is what
finally makes scan duration predictable rather than network-dependent.

### 4.3 The two "definitive" confirmations are not independent

`determine_publish_state` requires `git fetch` → `NotFound` **and** `gh repo view`
→ `NotFound`. Sound in intent, but GitHub returns **404, not 403**, for a private
repository you are not authenticated for. With expired credentials both checks
agree wrongly and every private repo is flagged deleted at once, then cached for
24 h. Fix: call `check_auth()` once per scan, store it on `RemoteCheckCtx`, and
skip the promotion entirely unless the status is `Ok`.

### 4.4 Failures still collapse into "clean"

`StatusChecker::check` converts every failure to `None` via `.ok()` /
`unwrap_or(false)`. `None` is not an error — `has_error` stays `false` — so the
repo lands in **Clean**. §2.1 stopped the *wrong* answers (`Ok(false)` on a
failed command); it did not add a way to say "could not check". That needs #10,
which is a real UX change (§6 Q4).

### 4.5 Concurrent folder scans clobber each other's cache

`scan_folder` loads the whole remote-check map at the start and writes the whole
map back at the end. Both frontends scan folders concurrently, so two
`RemoteCheckCtx` instances load the same snapshot and both write it back — last
writer wins. `write_atomic` prevents *corruption*, not *lost updates*, so the
24 h debounce never holds and `gh` re-runs every scan. Fix: hoist one
`RemoteCheckCtx` into `AppState` and persist by read-merge-write.

### 4.6 Remaining walk waste (independent of §6 Q1)

- `is_git_repo` costs a `PathBuf` allocation + a `stat` per directory, ~16,400
  times per scan. The `.git` entry is already yielded by the walk — detecting the
  repo from it costs zero extra syscalls.
- `uninitialized.rs` uses `is_file()` / `is_dir()` where `entry.file_type()` is
  already populated by `getdents`. Each call is a fresh `stat` that also follows
  symlinks.
- That symlink-following recursion has **no depth limit**, and `checked_dirs`
  stores literal (non-canonicalized) paths, so a symlink cycle produces
  infinitely many distinct keys → stack overflow or an unbounded hang. Latent
  today; a single symlinked project folder would trigger it. Switching to
  `file_type()` fixes it, since `WalkDir` is already `follow_links(false)`.
- O(n) prefix scans per directory entry in both `finder.rs` and
  `uninitialized.rs`; a sorted `Vec` + binary search makes both O(log n).

### 4.7 `has_pending_changes` is mostly well configured

`StatusOptions::new()` starts with flags at 0, so `INCLUDE_IGNORED` and
`RECURSE_UNTRACKED_DIRS` are off — untracked directories report as a single entry
and ignored trees like `node_modules` are skipped. Two costs remain:
`EXCLUDE_SUBMODULES` is off (libgit2 opens and diffs every submodule), and
`statuses()` builds the complete list before returning, so the early
`return Ok(true)` saves nothing.

Deliberately **not** recommended: `opts.update_index(true)`. It would speed up
later scans but writes `.git/index` once per repo per scan and risks `index.lock`
contention with any git process the user is running — surfacing as an
intermittent "Failed to check changes". Wrong trade when the goal is *less* I/O.

### 4.8 Settings are re-read once per repo during Clean All

Both bridges call `get_git_clean_settings()` per clean invocation, which does a
`read_to_string` + full JSON parse. Clean All over 60 repos = 60 reads and parses
of `settings.json`. Cache the parsed settings in `AppState` behind an `RwLock`,
invalidated on write.

---

## 5. Frontend parity

`FRONTEND.md` is the source of truth. The scan-orchestration divergences are now
resolved; what remains is presentation.

| Behavior | Status |
|---|---|
| Per-folder result delivery / spinner clearing | **Fixed** — both stream per folder |
| Bulk pull/clean error banner, "No ignored files to clean" | **Fixed** — both show it, before the rescan |
| Superseded scan's spinner | **Fixed** — both leave it to the owning scan |
| Blocking work off the runtime thread | **Fixed** — Tauri commands use `spawn_blocking` |
| Focus-throttle reference point | Open — Tauri measures from last scan *end*, macOS from last scan *start*. The spec is ambiguous; pick one |
| Sync settings commands on the main thread | Open — Tauri v2 runs sync commands on the main thread, and `get_available_terminals` / `get_available_editors` stat the whole app catalog there |
| List virtualization | Open — macOS uses a lazy `List`, Tauri a plain `.map`. Presentation only |

---

## 6. Open questions — these need your decision

1. **Are nested git repositories a supported feature?** The walk finds 7 in your
   tree (`gotutor/_playground/*`, `sumup-interview/*`) and lists them as
   independent top-level entries with no indication they are nested. Stopping the
   walk at a repo root takes it from **1.01 s to 0.05 s** and removes 7 network
   fetches — but those 7 repos disappear. Options: (a) prune always, (b) prune
   with an opt-in "find nested repos" setting, (c) keep as-is.
   **Recommendation: (b)** — a repo inside a repo is usually vendored or scratch
   work, but keep the escape hatch.

2. **Can ahead/behind be "as of last fetch"?** #9 is the largest remaining win:
   it removes 76 network calls from every startup, focus rescan and post-action
   rescan. The cost is that unpushed/unpulled counts go stale until an explicit
   "Refresh remotes" or a background TTL refresh.
   **Recommendation:** pair it with #8 so the local half paints in under a second
   and the network half fills in behind it. Right long-term shape, largest change.

3. **How should an unverifiable repo render?** (§4.4) A repo whose checks failed
   currently renders as Clean. Options: (a) a new `Unknown` state + badge,
   (b) keep the bucket but mark the row, (c) leave it.
   **Recommendation: (b)** — least disruptive, and it stops the silent lying.

---

## 7. Areas that are clean — no action needed

Recorded so they don't get re-investigated:

- **Ignore-pattern handling.** Applied as pruning *during* the walk, not post-hoc
  filtering; both sets are `LazyLock<HashSet<&'static str>>` compiled once per
  process; `to_string_lossy()` returns a borrowed `Cow` for valid UTF-8, so there
  is no per-entry allocation.
- **`RemoteCheckCtx` mutex discipline.** The guard is genuinely dropped before
  the `gh` call — under edition 2024 the `if let` scrutinee temporary ends with
  the statement. No lock is held across a subprocess.
- **Keyring/token access.** Once per process at startup, never in the scan path.
- **`reqwest` usage.** 15 s client-wide timeout, errors classified into
  `SyncOutcome::{Unauthorized, Network}`, degrades gracefully. Not on the scan path.
- **No nested parallelism**, no timers, no polling, no filesystem watchers, and no
  scan fired from a view's mount in either app.
- **`atomic_write`.** No fsync, which is the right call for advisory caches near
  the scan path — don't add one.
- **`classify_fetch`** correctly maps "could not resolve host" and auth failures to
  `Unreachable`, never `NotFound`, with tests.
- **The overlay-bucket design.** A repo appearing in both `unpublished` /
  `remote_not_found` and its exclusive bucket is intentional, documented, tested.
- **`UninitializedDetector`'s `HashSet` iteration.** Nondeterministic in order but
  verified result-deterministic: a directory reached by recursion has an ancestor
  with no repo at or under it, so the skipped `related_to_repo` check is a no-op
  there. Worth a comment, since one edit to either filter breaks the invariant.

---

## 8. Minor items, still open

- **Worktrees and submodules are invisible.** `is_git_repo` requires `.git` to be
  a **directory**, but linked worktrees and submodules have a `.git` **file**.
  They are neither detected as repos nor pruned, so their contents get walked
  *and* can be misreported as uninitialized folders.
- **The excluded-name check runs before the repo check** in `finder.rs`. A repo
  whose own folder is named `build`, `dist`, `out`, `bin`, `public`, `packages`
  or `gen` is invisible. Latent on your tree today, but a trap.
- **`RemoteNotFound` can't fire for repos with no upstream branch.**
  `check_remote_status` returns early, so `reachability` stays `None` and the
  promotion can never trigger. Saves work, but doesn't match the documented feature.
- **Cancellation is wired but unused, and wouldn't work.** `cancel_scan` has zero
  call sites. The flag is polled only in the finder, never in the status loop, so
  it would cut the walk but not the fetches. `cancel_scan` also takes
  `scanner.write()`, which blocks until the running scan releases its read lock —
  cancel waits for the scan it is cancelling. (It now at least blocks a blocking
  thread rather than a runtime worker.) And a cancelled walk produces a
  `ScanResult` indistinguishable from a complete one, which the frontends store as
  authoritative. All three need fixing before any cancel UI ships.
- **Overlapping monitored folders aren't rejected.** `add_folder` doesn't validate,
  so adding both `/Users/leo/Dev` and `/Users/leo/Dev/LeoManrique` would scan
  shared repos twice, concurrently, with two `git fetch` processes racing in the
  same `.git`.
- **`PathText` forces ~7 synchronous layouts per row.** A binary search calling
  `getBoundingClientRect()` at each step, per row, on mount and on every window
  resize — ~700 layout flushes at 100 rows. Not a scan-latency problem, but the
  reason resizing feels janky.
- **`visibleSections` is unmemoized**, and `RepoSection` / `RepoRow` aren't
  `React.memo`, so every keystroke in the search box re-renders every row.
