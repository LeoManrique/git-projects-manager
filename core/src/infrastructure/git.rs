use crate::infrastructure::process::output_with_timeout;
use anyhow::{Context, Result};
use git2::{Repository, Status, StatusOptions};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

/// Outcome of a `git fetch` against a repo's remote, classified from the exit
/// status and stderr. Only [`RemoteReachability::NotFound`] is a *definitive*
/// "the remote is gone" signal; connectivity/auth failures are `Unreachable`
/// and must never be treated as a deleted remote.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoteReachability {
    /// Fetch succeeded — the remote exists and is reachable.
    Reachable,
    /// The host reports the repository does not exist.
    NotFound,
    /// The fetch failed for a non-definitive reason (offline, DNS, auth, …).
    Unreachable,
}

/// Classify a `git fetch` result. Kept pure (no I/O) so it is unit-testable.
///
/// "Not found" is checked before anything else because some transports (SSH)
/// pair it with a generic "could not read from remote" line we'd otherwise
/// misread as a connectivity failure.
#[must_use]
pub fn classify_fetch(success: bool, stderr: &str) -> RemoteReachability {
    if success {
        return RemoteReachability::Reachable;
    }
    let s = stderr.to_lowercase();
    if s.contains("not found") || s.contains("does not exist") {
        RemoteReachability::NotFound
    } else {
        RemoteReachability::Unreachable
    }
}

/// Creates a git command with the invariants every call site depends on:
///
/// * `core.quotePath=false` — by default git C-quotes any path containing a
///   non-ASCII byte (`"caf\303\251/"`). [`GitOperations::clean`] joins the
///   printed path onto the repo root, so a quoted path resolved to a file that
///   does not exist and the whole clean failed.
/// * `LC_ALL=C` — we match on git's English output (`"Would remove "`), which
///   is translated on systems that ship git's `.mo` files.
/// * `GIT_TERMINAL_PROMPT=0` — `Command::output()` waits for EOF on the pipes,
///   so a credential prompt no GUI can answer would block the worker forever.
///
/// Also hides the console window on Windows.
fn git_command() -> Command {
    let mut cmd = Command::new("git");
    cmd.args(["-c", "core.quotePath=false"])
        .env("LC_ALL", "C")
        .env("GIT_TERMINAL_PROMPT", "0");
    #[cfg(target_os = "windows")]
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd
}

/// How long a single `git fetch` may run before it is killed.
///
/// The `-c` knobs below bound a *stalled transfer*, but not a TCP connect to a
/// black-holed route: libcurl's default connect timeout is 300 s. A killed
/// fetch classifies as [`RemoteReachability::Unreachable`] — never `NotFound` —
/// so the failure mode is "we could not check", not a repo wrongly flagged.
const FETCH_TIMEOUT: Duration = Duration::from_secs(20);

/// How long a *successful* fetch is trusted before the next scan re-fetches the
/// same repo.
///
/// Fetching stays on the scan path — ahead/behind counts are meant to be
/// current — but scans come in bursts that ask the same question twice: the
/// rescan fired right after a pull or clean, a window-focus rescan landing on
/// the heels of the startup scan, a Scan All moments after either. Those repeat
/// the whole network round-trip for a state that cannot have changed. The
/// window is deliberately short, so a scan the user asks for after doing
/// anything real is always a fresh one.
///
/// Only successes are recorded: a failed fetch must be retried immediately, not
/// cached.
const FETCH_DEBOUNCE: Duration = Duration::from_secs(30);

/// When each repo was last fetched successfully. Process-wide, because a
/// `Scanner` is replaced on cancel and every scan builds a fresh one.
static LAST_FETCH: LazyLock<Mutex<HashMap<PathBuf, Instant>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Whether `repo_path` was fetched recently enough to skip.
fn fetched_recently(repo_path: &Path) -> bool {
    let map = LAST_FETCH
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    map.get(repo_path)
        .is_some_and(|at| at.elapsed() < FETCH_DEBOUNCE)
}

fn record_fetch(repo_path: &Path) {
    let mut map = LAST_FETCH
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    // Bounded by "repos fetched in the last window" rather than every repo ever
    // seen, so a deleted or unmonitored folder does not linger.
    map.retain(|_, at| at.elapsed() < FETCH_DEBOUNCE);
    map.insert(repo_path.to_path_buf(), Instant::now());
}

/// One opened repository, reused for every local check the scanner runs.
///
/// Replaces four `git` subprocesses per repo (`remote`, `rev-parse @{upstream}`
/// and two `log <range> --oneline`) plus a second `Repository::open`. Measured
/// over 70 repos: the subprocesses cost 2.86 s of CPU, the libgit2 equivalents
/// 19 ms. Both `git log` calls ran a full revwalk and formatted output that was
/// then discarded — only its emptiness was ever read.
pub struct RepoInspector {
    repo: Repository,
}

impl RepoInspector {
    /// # Errors
    /// Returns an error if the path is not a repository libgit2 can open.
    pub fn open(repo_path: &Path) -> Result<Self> {
        Ok(Self {
            repo: Repository::open(repo_path)?,
        })
    }

    /// Short name of the checked-out branch, or `"HEAD"` when detached.
    ///
    /// # Errors
    /// Returns git2's `UnbornBranch` for an initialized repo with no commits —
    /// the caller distinguishes it via [`GitOperations::is_unborn_branch_error`].
    pub fn current_branch(&self) -> Result<String> {
        let head = self.repo.head()?;
        Ok(head.shorthand().unwrap_or("HEAD").to_string())
    }

    /// # Errors
    /// Returns an error if the working-tree status cannot be read.
    pub fn has_pending_changes(&self) -> Result<bool> {
        let mut opts = StatusOptions::new();
        opts.include_untracked(true);

        let statuses = self.repo.statuses(Some(&mut opts))?;
        Ok(statuses.iter().any(|e| e.status() != Status::CURRENT))
    }

    /// True if the repository has at least one remote configured. A repo with
    /// no remote has never been published to a host (see `ScanResult.unpublished`).
    ///
    /// # Errors
    /// Returns an error if the repository config cannot be read — never
    /// silently `false`, which would badge a published repo as Unpublished.
    pub fn has_remote(&self) -> Result<bool> {
        Ok(!self.repo.remotes()?.is_empty())
    }

    /// Full refname of HEAD's upstream (`refs/remotes/origin/main`), or `None`
    /// when there is no branch, no configured upstream, or the tracking ref no
    /// longer resolves.
    ///
    /// The resolve check is what `git rev-parse @{upstream}` did, and it must
    /// stay: without it a pruned tracking ref would still be "has upstream",
    /// and the scanner would fetch repos it used to skip.
    #[must_use]
    pub fn upstream_ref(&self) -> Option<String> {
        let head = self.repo.head().ok()?;
        let name = head.name().ok()?;
        let upstream = self.repo.branch_upstream_name(name).ok()?;
        let upstream = upstream.as_str().ok()?.to_string();
        self.repo.refname_to_id(&upstream).ok()?;
        Some(upstream)
    }

    /// `(has_unpushed, has_unpulled)` for HEAD against `upstream_ref`.
    ///
    /// Must be called *after* the fetch: libgit2 re-reads refs from disk (loose
    /// and packed alike), so one handle observes what an external `git fetch`
    /// just wrote — verified empirically before this was written.
    ///
    /// # Errors
    /// Returns an error if HEAD or the upstream ref cannot be resolved, rather
    /// than reporting "no commits either way" for a check that did not happen.
    pub fn ahead_behind(&self, upstream_ref: &str) -> Result<(bool, bool)> {
        let local = self
            .repo
            .head()?
            .target()
            .context("HEAD does not point at a commit")?;
        let upstream = self.repo.refname_to_id(upstream_ref)?;
        let (ahead, behind) = self.repo.graph_ahead_behind(local, upstream)?;
        Ok((ahead > 0, behind > 0))
    }
}

pub struct GitOperations;

impl GitOperations {
    #[must_use]
    pub fn is_git_repo(path: &Path) -> bool {
        path.join(".git").is_dir()
    }

    /// True if the error (anywhere in its chain) is git2's `UnbornBranch` —
    /// an initialized repository whose HEAD has no commits yet.
    #[must_use]
    pub fn is_unborn_branch_error(err: &anyhow::Error) -> bool {
        err.downcast_ref::<git2::Error>()
            .is_some_and(|g| g.code() == git2::ErrorCode::UnbornBranch)
    }

    /// Fetch from the remote, reporting whether it is reachable, gone, or
    /// merely unreachable (see [`RemoteReachability`]).
    ///
    /// # Errors
    /// Returns an error if the `git fetch` command cannot be spawned, or if it
    /// exceeded [`FETCH_TIMEOUT`] and was killed.
    pub fn fetch(repo_path: &Path) -> Result<RemoteReachability> {
        // Recently fetched: the tracking refs on disk are at most
        // `FETCH_DEBOUNCE` old, and ahead/behind is still computed from them, so
        // the counts stay current — only the round-trip is skipped. Reported as
        // `Reachable` because that is what the skipped fetch established.
        if fetched_recently(repo_path) {
            return Ok(RemoteReachability::Reachable);
        }

        let mut cmd = git_command();
        cmd.args([
            // A scan runs this once per repo. `--auto` maintenance would fork a
            // second process per repo to repack in the background, which is the
            // main reason one scan is inexplicably slower than the last.
            "-c",
            "gc.auto=0",
            "-c",
            "maintenance.auto=false",
            // Abort a transfer that has stalled below 1 KiB/s for 20s. libcurl's
            // default connect timeout is 300s, so without this a black-holed
            // route holds a worker for minutes and the scan appears frozen.
            "-c",
            "http.lowSpeedLimit=1000",
            "-c",
            "http.lowSpeedTime=20",
            "fetch",
            "--quiet",
            // Nothing in a scan result depends on tags or submodule refs, and
            // both are pure extra network and disk work.
            "--no-tags",
            "--no-recurse-submodules",
        ])
        .current_dir(repo_path);

        // The SSH equivalent of the two settings above. Skipped when the user
        // has configured their own ssh wrapper, which we must not override.
        if std::env::var_os("GIT_SSH_COMMAND").is_none() {
            cmd.env("GIT_SSH_COMMAND", "ssh -oBatchMode=yes -oConnectTimeout=10");
        }

        let output = output_with_timeout(&mut cmd, FETCH_TIMEOUT)?;

        let stderr = String::from_utf8_lossy(&output.stderr);
        let reachability = classify_fetch(output.status.success(), &stderr);
        if reachability == RemoteReachability::Reachable {
            record_fetch(repo_path);
        }
        Ok(reachability)
    }

    /// `(has_unpushed, has_unpulled)` via `git rev-list`, the fallback for when
    /// libgit2's in-process graph walk cannot answer.
    ///
    /// libgit2 has real gaps the CLI does not: it ignores `refs/replace/*`
    /// entirely, and its commit-graph reader bypasses the shallow/graft
    /// boundary that git itself refuses to combine with a commit-graph. Those
    /// surface as an error from `graph_ahead_behind`, and an error used to mean
    /// the repo silently landed in Clean. One subprocess, only for the repos
    /// that already failed, turns "wrong" back into "slower".
    ///
    /// # Errors
    /// Returns an error if `git rev-list` cannot be executed, exits with a
    /// failure status, or prints something other than two counts.
    pub fn ahead_behind_via_cli(repo_path: &Path) -> Result<(bool, bool)> {
        let output = git_command()
            .args(["rev-list", "--left-right", "--count", "@{upstream}...HEAD"])
            .current_dir(repo_path)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("git rev-list failed: {}", stderr.trim());
        }

        // "<behind>\t<ahead>": commits reachable from the upstream but not
        // HEAD, then the reverse.
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut counts = stdout.split_whitespace();
        let behind: u32 = counts.next().context("no behind count")?.parse()?;
        let ahead: u32 = counts.next().context("no ahead count")?.parse()?;
        Ok((ahead > 0, behind > 0))
    }

    /// # Errors
    /// Returns an error if the `git pull` command cannot be executed, or if it
    /// exits with a failure status.
    pub fn pull(repo_path: &Path) -> Result<String> {
        // No explicit fetch first: `git pull` *is* fetch + merge, so the extra
        // call was a second full network round-trip whose result was discarded.
        let output = git_command()
            .arg("pull")
            .current_dir(repo_path)
            .output()?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            let error = String::from_utf8_lossy(&output.stderr).to_string();
            anyhow::bail!("Pull failed: {error}")
        }
    }

    /// Clean ignored files from the repository, preserving paths that match
    /// any of the user-provided exclude patterns.
    ///
    /// `git clean -X` itself has no way to *preserve* matches — its `-e` flag
    /// adds patterns to the ignore set, which is the opposite of what we want.
    /// So we dry-run, filter in Rust, then delete the survivors ourselves.
    ///
    /// # Errors
    /// Returns an error if the `git clean` dry run cannot be executed or exits
    /// with a failure status. If individual paths cannot be deleted the rest
    /// are still removed, and the error names every path that failed.
    pub fn clean(repo_path: &Path, exclude_patterns: &[String]) -> Result<(Vec<String>, Vec<String>)> {
        let mut cmd = git_command();
        cmd.arg("clean")
            .arg("-fdXn") // dry run: list what would be removed
            .current_dir(repo_path);

        let output = cmd.output()?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr).to_string();
            anyhow::bail!("Clean failed: {error}")
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut files_removed = Vec::new();
        let mut directories_removed = Vec::new();
        let mut failures: Vec<String> = Vec::new();

        for line in stdout.lines() {
            let Some(path) = line.strip_prefix("Would remove ") else {
                continue;
            };

            if path_matches_any(path, exclude_patterns) {
                continue;
            }

            let is_dir = path.ends_with('/');
            let clean_path = path.trim_end_matches('/');
            let abs = repo_path.join(clean_path);

            let removed = if is_dir {
                std::fs::remove_dir_all(&abs)
            } else {
                std::fs::remove_file(&abs)
            };

            // One unremovable path must not abandon the rest of the repo. A
            // build, a watcher or a language server can delete a listed path
            // between the dry run and here, and a single permission error deep
            // in a tree used to surface as a total failure naming nothing.
            match removed {
                Ok(()) => {
                    if is_dir {
                        directories_removed.push(clean_path.to_string());
                    } else {
                        files_removed.push(path.to_string());
                    }
                }
                // Already gone is the outcome we wanted, not a failure.
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => failures.push(format!("{clean_path} ({e})")),
            }
        }

        if !failures.is_empty() {
            anyhow::bail!(
                "removed {} path(s), failed to remove {}: {}",
                files_removed.len() + directories_removed.len(),
                failures.len(),
                failures.join("; ")
            );
        }

        Ok((files_removed, directories_removed))
    }
}

/// Returns true if `path` matches any of the given glob patterns.
fn path_matches_any(path: &str, patterns: &[String]) -> bool {
    patterns.iter().any(|p| path_matches(path, p))
}

/// Matches a relative path against a single gitignore-style pattern.
///
/// A trailing `/` on the pattern restricts the match to directory components.
/// The pattern is tested against each segment of the path and, for non-dir
/// patterns, against the full path string. Supports `*` and `?` wildcards.
fn path_matches(path: &str, pattern: &str) -> bool {
    let dir_only = pattern.ends_with('/');
    let pat = pattern.trim_end_matches('/');
    let path_is_dir = path.ends_with('/');
    let clean = path.trim_end_matches('/');

    if dir_only {
        let segments: Vec<&str> = clean.split('/').collect();
        let dir_count = if path_is_dir { segments.len() } else { segments.len().saturating_sub(1) };
        segments[..dir_count].iter().any(|s| glob_match(pat, s))
    } else {
        clean.split('/').any(|s| glob_match(pat, s)) || glob_match(pat, clean)
    }
}

/// Iterative glob matcher supporting `*` (any chars) and `?` (one char).
fn glob_match(pattern: &str, name: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let n: Vec<char> = name.chars().collect();
    let (mut pi, mut ni) = (0, 0);
    let mut star_pat: Option<usize> = None;
    let mut star_name = 0;

    while ni < n.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == n[ni]) {
            pi += 1;
            ni += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star_pat = Some(pi);
            star_name = ni;
            pi += 1;
        } else if let Some(sp) = star_pat {
            pi = sp + 1;
            star_name += 1;
            ni = star_name;
        } else {
            return false;
        }
    }

    while pi < p.len() && p[pi] == '*' {
        pi += 1;
    }

    pi == p.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glob_matches_basic() {
        assert!(glob_match(".env*", ".env.prod"));
        assert!(glob_match(".env*", ".env"));
        assert!(glob_match("*.key", "private.key"));
        assert!(glob_match("*.pem", "cert.pem"));
        assert!(!glob_match(".env*", "env.prod"));
        assert!(!glob_match("*.key", "key.txt"));
    }

    #[test]
    fn path_matches_env_pattern() {
        let patterns = vec![".env*".to_string()];
        assert!(path_matches_any(".env.prod", &patterns));
        assert!(path_matches_any("relay/.env.prod", &patterns));
        assert!(!path_matches_any("env.prod", &patterns));
        assert!(!path_matches_any("relay/env.prod", &patterns));
    }

    #[test]
    fn path_matches_dir_pattern() {
        let patterns = vec![".vscode/".to_string()];
        assert!(path_matches_any(".vscode/settings.json", &patterns));
        assert!(path_matches_any(".vscode/", &patterns));
        assert!(path_matches_any("sub/.vscode/foo", &patterns));
    }

    #[test]
    fn classify_fetch_success_is_reachable() {
        assert_eq!(classify_fetch(true, ""), RemoteReachability::Reachable);
    }

    #[test]
    fn classify_fetch_detects_deleted_remote() {
        // GitHub HTTPS
        assert_eq!(
            classify_fetch(false, "remote: Repository not found.\nfatal: repository 'https://github.com/x/y.git/' not found"),
            RemoteReachability::NotFound
        );
        // GitHub SSH pairs "not found" with a generic connectivity line.
        assert_eq!(
            classify_fetch(false, "ERROR: Repository not found.\nfatal: Could not read from remote repository."),
            RemoteReachability::NotFound
        );
    }

    #[test]
    fn classify_fetch_treats_connectivity_and_auth_as_unreachable() {
        assert_eq!(
            classify_fetch(false, "fatal: unable to access '...': Could not resolve host: github.com"),
            RemoteReachability::Unreachable
        );
        assert_eq!(
            classify_fetch(false, "fatal: Authentication failed for 'https://github.com/x/y.git/'"),
            RemoteReachability::Unreachable
        );
    }
}
