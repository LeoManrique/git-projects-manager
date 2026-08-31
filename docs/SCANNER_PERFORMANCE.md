# Scanner Performance & Reliability Audit

Why scans were slow, why results were inconsistent between runs, and where I/O
and network work can still be removed. Kanban is out of scope.

**Status: closed.** Tiers 1 and 2 are implemented, the three product decisions
in §6 have been taken and built, and the correctness guards in §4 are done.
Nothing performance-related remains that measurement justifies — §4.4 records
what was checked and left alone. `DESIGN.md`, `FRONTEND.md` and `TECHNICAL.md`
describe the behavior as it is now; this document keeps the reasoning, the
measurements, and the corrections.

Measure with `just bench-scan <path> [local]`, which prints three timed runs
plus every bucket count — so a change can be shown to be faster *and* to still
find the same repos.

---

## 1. Measured results

`/Users/leo/Dev`, 76 repos, 12 logical CPUs, all-HTTPS remotes. "Warm" means a
populated OS page cache; the first scan after boot costs ~0.8 s more in the walk.

| | Original | After Tier 1 | After Tier 2 | With the debounce |
|---|---|---|---|---|
| Online scan, cold | 4.7 s | 2.3 s | 2.1–2.8 s | 2.1–3.4 s |
| Online **rescan** (within 30 s) | 4.7 s | 2.3 s | 2.1–2.8 s | **1.0 s** |
| Local-only scan | — | 0.66 s | **0.46 s** | 0.46 s |
| Status phase, local-only | — | 375 ms | **215 ms** | 215 ms |
| Bucket counts | — | identical | identical | identical |

Two findings worth stating plainly:

**Tier 2's throughput work does not speed up a cold online scan.** It is bound
by 76 `git fetch` round-trips; removing ~2.9 s of local CPU work disappears into
network noise. Tier 2's value is elsewhere — local-only scans 30% faster, a
bounded worst case, and several silent miscategorizations gone.

**The win came from removing repeated work, not faster work.** Every draft of
this audit framed the problem as "76 fetches are too many" and reached for
staleness to fix it. The actual waste was those same 76 fetches running three
times in a row, because a pull, a window focus and a startup scan each trigger
their own rescan seconds apart. Debouncing a successful fetch for 30 s cuts a
repeat scan to **1.0 s** while keeping every count current (§6 Q2).

Phase breakdown of a warm local scan (measured, then the instrumentation removed):

| Phase | Cost | Note |
|---|---|---|
| Directory walk | 240 ms | 1.0 s cold; 81% of it is inside repos already found (§6 Q1) |
| Uninitialized detection | **1.5 ms** | see §3 — this was a planned optimization target |
| Status checks (76 repos) | 215 ms | was 375 ms |

Local operations, measured serially over 70 repos:

| | Subprocess | libgit2 | |
|---|---|---|---|
| `git remote` | 659 ms | 0.55 ms | reusing one open handle |
| `rev-parse @{upstream}` + 2× `git log` | 2.20 s | 12 ms | one `graph_ahead_behind` |
| `Repository::open` × 76 | — | 4.2 ms | ~56 µs each |

---

## 2. What changed

Tier 1 (thread pool, fetch hardening, scoped rescans, the `git clean` parser,
error surfacing in both frontends) is described in `TECHNICAL.md` and
`FRONTEND.md`. Tier 2:

| # | Change | Where |
|---|---|---|
| 1 | One `RepoInspector` (libgit2 handle) replaces 4 subprocesses per repo | [git.rs](../core/src/infrastructure/git.rs) |
| 2 | Wall-clock timeout + kill for `git fetch` (20 s) and `gh` (15 s) | [process.rs](../core/src/infrastructure/process.rs) |
| 3 | `RemoteNotFound` gated on one `check_auth()` per scan | [remote_check.rs](../core/src/domain/scanner/remote_check.rs) |
| 5 | Remote-check cache persists by read-merge-write | [remote_check_store.rs](../core/src/infrastructure/remote_check_store.rs) |
| 6 | `gh` resolved once, then invoked directly with argv | [github_cli.rs](../core/src/infrastructure/github_cli.rs) |
| 7 | Remote-check cache prunes entries for paths that are gone | [remote_check_store.rs](../core/src/infrastructure/remote_check_store.rs) |
| — | Uninitialized walk no longer follows symlinks | [uninitialized.rs](../core/src/domain/scanner/uninitialized.rs) |

**The two `git log` calls were the expensive ones.** Each ran a full revwalk and
formatted `--oneline` output that was then thrown away — only `is_empty()` was
ever read. `graph_ahead_behind` answers the same question 180× faster.

**The timeout helper drains both pipes on their own threads.** `try_wait` never
reads them, so a child that fills the 64 KiB pipe buffer blocks forever waiting
for a reader that only appears after it exits — a poll loop written the obvious
way deadlocks against the very thing it is timing out. A killed fetch classifies
as `Unreachable`, never `NotFound`, so a timeout can never flag a live repo as
deleted.

**The auth gate closes a real false-positive class.** GitHub answers **404, not
403**, for a private repository you are not authenticated for — and `git fetch`
says "not found" for the same reason. So the two supposedly independent
confirmations agree *wrongly* the moment credentials expire, and every private
repo is flagged deleted at once, then cached for 24 h. `check_auth` is now
consulted before the cache, because a verdict recorded while unauthenticated is
exactly the one not to trust. It is resolved lazily, so a scan that never
reaches the promotion path never pays for it.

**Symlinks in the uninitialized walk.** The recursion has no depth limit and
`checked_dirs` keys on the literal path, so a symlink back to an ancestor
produced a fresh key at every level. `DirEntry::file_type` (no syscall, does not
follow symlinks) replaces `Path::is_dir`, which also aligns this walk with
`RepositoryFinder`'s `follow_links(false)`.

### 2.1 Hardening added after an adversarial review of the libgit2 swap

A dedicated review compared each libgit2 replacement against its `git` CLI
predecessor. Two mitigations came out of it, both in
[status_checker.rs](../core/src/domain/scanner/status_checker.rs):

- **The handle is re-opened after the fetch.** Refs and objects are re-read per
  lookup (verified empirically, packed-refs included), but a few things load
  once at open time — notably the shallow-clone boundary in `.git/shallow`,
  which a fetch can rewrite. Defensive rather than a fixed bug: see §3.
- **`git rev-list` is the fallback when libgit2's graph walk errors.** libgit2
  ignores `refs/replace/*` entirely, and its commit-graph reader bypasses the
  shallow/graft boundary that git itself refuses to combine with a commit-graph.
  Those surface as an error, and an error used to mean the repo silently landed
  in **Clean**. One subprocess, only for repos that already failed, turns
  "wrong" back into "slower".

### 2.2 Test coverage added

14 new tests, all offline (a local bare repo stands in for the remote):

- `ahead_behind.rs` — unpulled / unpushed / in-sync / no-upstream / shallow /
  detached HEAD / unborn / pruned tracking ref. The unpulled case only passes if
  the in-scan fetch advances the tracking ref *and* the handle observes it.
- `uninitialized_symlinks.rs` — a symlink cycle, and a symlinked project folder.
  Both fail against the previous code.
- `process.rs` unit tests — a fast command, a killed one, and 300 KB of output
  (the deadlock case).

The §4.0 guards added 11 more: `repo_folder_names.rs` (a repo named after each
excluded directory, a real build output still pruned, a vendored `lib` still
hidden) and the `folder.rs` overlap unit tests, including the sibling-prefix
case (`/a/development` is not inside `/a/dev`) that a string comparison would
get wrong.

---

## 3. Corrections to this audit's own claims

Recorded because each was asserted confidently and was wrong. The first three
were found while implementing Tier 2.

- **Item #4 — "fold the uninitialized walk into the main one" — is not worth
  doing.** It was described as removing "a second and third walk of the same
  tree". Measured: **1.5 ms** out of a 460 ms scan, 0.3%. It only ever visits
  directories that are *not* repos and *not* inside one, which in this tree is
  almost nothing. Dropped from the plan; the file was instead fixed for the
  symlink bug that actually mattered.
- **The "1.01 s directory walk" was a cold measurement.** Warm it is 240 ms.
  81% of it is still waste, so §6 Q1 stands, but the steady-state prize is
  ~0.2 s, not ~1 s.
- **Item #6 saves ~52 ms per `gh` call, not "100–500 ms".** A login shell costs
  70 ms per call against 19 ms for a direct exec. But a `gh repo view` is ~400 ms
  end to end — it is bound by GitHub's API, not the shell — so this is ~13% of
  one call, and `gh` runs at most a handful of times per scan. The change was
  kept for *determinism*, not speed: results no longer depend on what the login
  profile happens to set, and argv replaces an interpolated shell string.
- **Item #5's proposed fix was the wrong one.** Hoisting one `RemoteCheckCtx`
  into `AppState` would fix two folder scans racing inside one app, but not the
  two apps racing with each other. Making the *store* do a locked
  read-merge-write (newest `checked_at` wins) fixes both, and is smaller.
- **The symlink hazard is not a hang.** It was described as unbounded recursion
  ending in a stack overflow. The OS refuses to resolve a symlink chain past its
  own limit (`ELOOP`), and the failed `read_dir` ends the recursion. The real
  symptom is duplicates: the same project folder reported **16 times**, once per
  level, measured.
- **`EXCLUDE_SUBMODULES` should stay off.** §4.7 listed it as waste. Four repos
  in this tree have submodules, and `git status` reports a dirty submodule as a
  change to the parent — so enabling it would change results, not just cost.
- **Re-reading settings per clean costs single-digit milliseconds.** §4.8 framed
  60 reads and parses of `settings.json` as a problem; it is a small file and
  the parse is ~50 µs. Not worth caching.
- **Submodules cannot be misreported as uninitialized folders.** §8 claimed a
  `.git`-file repo is "neither detected as a repo nor pruned, so its contents get
  walked *and* can be misreported as uninitialized". The second half is
  impossible: `should_check_directory` skips any path where
  `path.starts_with(repo)`, and a submodule is by definition inside its
  superproject. The claim only holds for a *linked worktree*, which lives
  outside the repo it belongs to. Measured here: 5 submodules, 0 worktrees, and
  the submodules cost ~72 of ~16,400 walked directories.
- **The stale-shallow-graft risk is theoretical.** The review reasoned from
  libgit2's source that grafts load once at open. No failing case could be
  constructed: the merge base between a branch and its own upstream is recent,
  and a shallow clone has no deep local history to walk past. The re-open is
  kept anyway — 60 µs against a silent miscategorization is a trade worth making
  — but it is defense in depth, not a fixed bug.

Earlier corrections, from the Tier 1 pass, still stand: `git clean` has no `-z`
flag; `LC_ALL=C` is a no-op on Apple Git but kept for Linux; the five
same-timestamp `exists: false` cache entries were genuinely deleted repos, which
is what motivated item #7; the `gh` "command not found" bug is latent here, not
active; and `finder.rs` does prune, just not at repo roots.

---

## 4. Correctness guards (done) and what was left alone

The remaining work was never about speed. Each item below was measured against
the real tree first; three were fixed because the failure mode is silence, and
the rest were left alone because measurement said they were not worth it.

### 4.0 Fixed

| Guard | Why | Where |
|---|---|---|
| A repo is no longer hidden by its own folder name | The excluded-name prune ran before the repo check, so a repo named `build`, `dist`, `packages`, `public`, `bin`, `gen` or `out` produced no entry and no error. A fixture of seven such repos found **none** of them against the old code | [finder.rs](../core/src/domain/scanner/finder.rs) |
| Overlapping monitored folders are rejected | Adding `~/Dev` and `~/Dev/LeoManrique` scanned shared repos twice in one pass, with two `git fetch` processes racing in one `.git`. Compared by path component after `canonicalize`, so `/a/bc` is not "inside" `/a/b` | [folder.rs](../core/src/domain/folder.rs), [config_store.rs](../core/src/infrastructure/config_store.rs) |
| `cancel_scan` removed | See below | [scanner.rs](../core/src/domain/scanner.rs) |

`cancel_scan` was exposed through both frontends' API layers with zero callers,
and all three of its problems were fatal: the flag was polled only in the
directory walk, so it stopped the cheap half of a scan and left every `git fetch`
running; `cancel_scan` took `scanner.write()`, which blocks until the running
scan releases its read lock, so cancel waited for the scan it was cancelling; and
a cancelled walk produced a `ScanResult` indistinguishable from a complete one,
which the frontends stored as authoritative. Deleting it also made `Scanner`
stateless, so `AppState` holds an `Arc<Scanner>` instead of `Arc<RwLock<Scanner>>`.
A cancel UI needs polling in the status loop and a partial-result marker first.

Both frontends now surface the core's own error message from the folder form
instead of a generic "Failed to add folder", since the overlap rejection is only
useful if it can say what it collided with.

### 4.1 A pruned tracking ref still reads as Clean

The remaining case where the scan asserts more than it knows. After a PR merge
GitHub deletes the branch, `fetch.prune` removes `refs/remotes/origin/X`, and
`branch.X.remote`/`branch.X.merge` remain. `@{upstream}` then resolves for
neither libgit2 nor the CLI, so `upstream_ref()` returns `None`, the repo is
never probed, and it lands in **Clean** even with unpushed commits.

Deliberately *not* covered by `remote_state_unknown`, which means "asked and
failed". Here nothing failed — there is no upstream to ask about. Treating it as
unknown would also flag every genuinely upstream-less branch. Fixing it properly
means falling back to comparing against `refs/remotes/<remote>/<branch>` by name,
or reporting "upstream branch is gone" as its own state. Pre-existing, verified
identical in the old code, and covered by a test so the behavior is deliberate
rather than accidental. **Zero occurrences** in this tree when checked, so it
stays open on judgment rather than evidence.

### 4.2 Remaining walk waste (independent of §6 Q1)

- `is_git_repo` costs a `PathBuf` allocation + a `stat` per directory, ~16,400
  times per scan. The `.git` entry is already yielded by the walk. **Measured at
  ~25 ms of 240 ms**, and detecting the repo from the `.git` entry instead would
  make the nested-exclusion logic depend on readdir order. Not worth it without
  §6 Q1.
- O(n) prefix scans per directory entry in both `finder.rs` and
  `uninitialized.rs`; a sorted `Vec` + binary search makes both O(log n). Only
  matters at a much larger repo count than this tree has.

### 4.3 Accepted differences from the `git` CLI

Both from the adversarial review, both verified, both deliberately not "fixed":

- **A remote with no URL.** `git remote` lists any `remote.<name>.*` key;
  libgit2 lists only remotes that have a `url` or `pushurl`. So a repo left with
  only `remote.origin.fetch` now reads as Unpublished. For the question the
  badge actually asks — "has this been published to a host?" — libgit2's answer
  is the better one. Zero occurrences in this tree.
- **Partial clones and reftable repos cannot be opened by libgit2 at all**
  (`extensions.partialclone`, `extensions.refstorage`), so they land in
  `errors`. Pre-existing — the old code also opened a `Repository` — and zero
  exposure today, but one `git clone --filter=blob:none` changes that.

### 4.4 Checked and deliberately not done

Each of these was in an earlier "still open" list and was dropped after being
measured against `~/Dev`:

| Item | Measurement | Verdict |
|---|---|---|
| `is_git_repo` allocation + `stat` per directory | 25 ms of a 240 ms walk | The alternative — detecting the repo from the `.git` entry the walk already yields — makes nested exclusion depend on readdir order |
| O(n) prefix scans per entry in `finder.rs` / `uninitialized.rs` | Invisible at 76 repos | A sorted `Vec` + binary search only pays off at a much larger tree |
| Worktrees and submodules invisible (`.git` is a **file**) | 5 submodules, 0 worktrees; **~72 of ~16,400** directories walked | Not showing a submodule as its own project is arguably correct. See the correction in §3 |
| Repo folder named `build`, `dist`, … | 0 today | **Fixed anyway** (§4.0) — the failure is silent, so waiting for a first occurrence means never noticing it |

---

## 5. Frontend parity

`FRONTEND.md` is the source of truth. Scan orchestration is aligned; what
remains is presentation. Unchanged by Tier 2.

| Behavior | Status |
|---|---|
| Per-folder result delivery / spinner clearing | **Fixed** |
| Bulk pull/clean error banner, "No ignored files to clean" | **Fixed** |
| Superseded scan's spinner | **Fixed** |
| Blocking work off the runtime thread | **Fixed** |
| Focus-throttle reference point | Open — Tauri measures from last scan *end*, macOS from last scan *start*. The spec is ambiguous; pick one |
| Sync settings commands on the main thread | Open — Tauri v2 runs sync commands on the main thread, and `get_available_terminals` / `get_available_editors` stat the whole app catalog there |
| List virtualization | Open — macOS uses a lazy `List`, Tauri a plain `.map`. Presentation only |

---

## 6. Decisions taken

1. **Nested repositories stay supported.** The walk does not stop at a repo root,
   so the 7 nested repos in this tree keep their entries. Cost accepted: ~190 ms
   of walk per scan and 7 extra fetches. Item removed from §4.

2. **Fetch stays on every scan, with a short debounce.** Counts must be current,
   so taking `fetch` off the scan path (items 8 and 9) is dropped. Instead a
   *successful* fetch is remembered per repo for **30 seconds** and reused inside
   that window. This targets the redundancy rather than the work: scans come in
   bursts — the rescan after a pull or clean, a focus rescan on the heels of the
   startup scan — that repeat the same round-trips for a state that cannot have
   changed. Measured over 76 repos, a repeat scan goes **3.4 s → 1.0 s**. Failed
   fetches are never cached, so an unreachable remote retries immediately.

   This turned out to be the largest practical win of the whole audit, and it is
   one the earlier drafts never proposed: they framed the problem as "76 fetches
   are too many" and reached for staleness, when the actual waste was *the same
   76 fetches happening three times in a row*.

3. **A repo whose remote comparison failed gets its own overlay.** New
   `remote_state_unknown` bucket — "Unknown Remote State", gray, Fetch & Pull
   still offered — sitting on top of the exclusive bucket like the other two
   overlays. It fires only when both the libgit2 walk *and* the `git rev-list`
   fallback fail.

   Paired with it, because the distinction is the whole point: a
   **"Local checks only" chip** beside Scan All in both apps. An
   `onlyLocalChecks` folder reports no unpushed and no unpulled work for every
   repo simply because it never asked, which is indistinguishable on screen from
   being up to date. That is "did not check", and the Unknown section
   deliberately does not claim it — `remote_state_unknown` is always empty for
   those folders.

---

## 7. Areas that are clean — no action needed

Recorded so they don't get re-investigated:

- **Ignore-pattern handling.** Applied as pruning *during* the walk, not post-hoc
  filtering; both sets are `LazyLock<HashSet<&'static str>>` compiled once per
  process; `to_string_lossy()` returns a borrowed `Cow` for valid UTF-8.
- **`RemoteCheckCtx` mutex discipline.** The guard is genuinely dropped before
  the `gh` call — under edition 2024 the `if let` scrutinee temporary ends with
  the statement. No lock is held across a subprocess.
- **Keyring/token access.** Once per process at startup, never in the scan path.
- **`reqwest` usage.** 15 s client-wide timeout, errors classified into
  `SyncOutcome::{Unauthorized, Network}`. Not on the scan path.
- **No nested parallelism**, no timers, no polling, no filesystem watchers, and no
  scan fired from a view's mount in either app.
- **`atomic_write`.** No fsync, which is the right call for advisory caches near
  the scan path — don't add one.
- **`classify_fetch`** correctly maps "could not resolve host" and auth failures to
  `Unreachable`, never `NotFound`, with tests.
- **The overlay-bucket design.** A repo appearing in both `unpublished` /
  `remote_not_found` and its exclusive bucket is intentional, documented, tested.
- **`has_pending_changes` option flags.** `StatusOptions::new()` starts at 0, so
  `INCLUDE_IGNORED` and `RECURSE_UNTRACKED_DIRS` are off — untracked directories
  report as one entry and ignored trees like `node_modules` are skipped.
  Deliberately **not** adding `update_index(true)`: it would speed up later scans
  but writes `.git/index` once per repo per scan and risks `index.lock`
  contention with the user's own git processes.
- **`UninitializedDetector`'s `HashSet` iteration.** Nondeterministic in order but
  verified result-deterministic: a directory reached by recursion has an ancestor
  with no repo at or under it, so the skipped `related_to_repo` check is a no-op
  there. Worth a comment, since one edit to either filter breaks the invariant.

---

## 8. Minor items, still open

- **Linked worktrees are invisible.** `is_git_repo` requires `.git` to be a
  **directory**, but a linked worktree has a `.git` **file**. It is neither
  detected as a repo nor pruned, so it is walked and can be reported as an
  uninitialized folder. (Submodules share the `.git`-file shape but not the
  consequence — see §3.) Zero worktrees in this tree.
- **`RemoteNotFound` can't fire for repos with no upstream branch.**
  `check_remote_status` returns early, so `reachability` stays `None` and the
  promotion can never trigger. Saves work, but doesn't match the documented
  feature.
- **Triangular workflows measure "unpushed" against the wrong remote.** Both the
  old and new code compare against `@{upstream}`, never `@{push}`. Pre-existing.
- **`PathText` forces ~7 synchronous layouts per row.** A binary search calling
  `getBoundingClientRect()` at each step, per row, on mount and on every window
  resize — ~700 layout flushes at 100 rows. Not a scan-latency problem, but the
  reason resizing feels janky.
- **`visibleSections` is unmemoized**, and `RepoSection` / `RepoRow` aren't
  `React.memo`, so every keystroke in the search box re-renders every row.
