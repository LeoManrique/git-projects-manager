use anyhow::Result;
use git2::{Repository, Status, StatusOptions};
use std::path::Path;
use std::process::Command;

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

    /// # Errors
    /// Returns an error if the repository cannot be opened or HEAD cannot be read.
    pub fn get_current_branch(repo_path: &Path) -> Result<String> {
        let repo = Repository::open(repo_path)?;
        let head = repo.head()?;

        if let Ok(name) = head.shorthand() {
            Ok(name.to_string())
        } else {
            Ok("HEAD".to_string())
        }
    }

    /// # Errors
    /// Returns an error if the repository cannot be opened or its status cannot be read.
    pub fn has_pending_changes(repo_path: &Path) -> Result<bool> {
        let repo = Repository::open(repo_path)?;
        let mut opts = StatusOptions::new();
        opts.include_untracked(true);

        let statuses = repo.statuses(Some(&mut opts))?;

        for entry in statuses.iter() {
            let status = entry.status();
            if status != Status::CURRENT {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// # Errors
    /// Returns an error if the `git log` command cannot be executed or exits
    /// with a failure status.
    pub fn has_unpushed_commits(repo_path: &Path) -> Result<bool> {
        // Using git command for simplicity as git2 branch tracking is complex
        let output = git_command()
            .arg("log")
            .arg("@{upstream}..HEAD")
            .arg("--oneline")
            .current_dir(repo_path)
            .output()?;

        commits_in_range(&output, "@{upstream}..HEAD")
    }

    /// True if the repository has at least one remote configured. A repo with
    /// no remote has never been published to a host (see `ScanResult.unpublished`).
    ///
    /// # Errors
    /// Returns an error if the `git remote` command cannot be executed or
    /// exits with a failure status.
    pub fn has_remote(repo_path: &Path) -> Result<bool> {
        let output = git_command()
            .arg("remote")
            .current_dir(repo_path)
            .output()?;

        // An empty stdout means "no remotes" only if git actually succeeded.
        // On failure (dubious ownership, an unreadable config) stdout is also
        // empty, and returning `false` there produced an "Unpublished" badge
        // for a repo that is published.
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("git remote failed: {}", stderr.trim());
        }

        Ok(!String::from_utf8_lossy(&output.stdout).trim().is_empty())
    }

    /// # Errors
    /// Returns an error if the `git rev-parse` command cannot be executed.
    pub fn has_upstream_branch(repo_path: &Path) -> Result<bool> {
        let output = git_command()
            .arg("rev-parse")
            .arg("--abbrev-ref")
            .arg("--symbolic-full-name")
            .arg("@{upstream}")
            .current_dir(repo_path)
            .output()?;

        Ok(output.status.success())
    }

    /// Fetch from the remote, reporting whether it is reachable, gone, or
    /// merely unreachable (see [`RemoteReachability`]).
    ///
    /// # Errors
    /// Returns an error if the `git fetch` command cannot be spawned.
    pub fn fetch(repo_path: &Path) -> Result<RemoteReachability> {
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

        let output = cmd.output()?;

        let stderr = String::from_utf8_lossy(&output.stderr);
        Ok(classify_fetch(output.status.success(), &stderr))
    }

    /// # Errors
    /// Returns an error if the `git log` command cannot be executed or exits
    /// with a failure status.
    pub fn has_unpulled_commits(repo_path: &Path) -> Result<bool> {
        let output = git_command()
            .arg("log")
            .arg("HEAD..@{upstream}")
            .arg("--oneline")
            .current_dir(repo_path)
            .output()?;

        commits_in_range(&output, "HEAD..@{upstream}")
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

/// Interpret the output of a `git log <range> --oneline` used purely as an
/// emptiness test.
///
/// A non-zero exit also produces empty stdout — the upstream ref was pruned by
/// the fetch that just ran, the branch was renamed on the remote, HEAD is
/// detached — so reading stdout alone reported "no commits in range" as fact
/// and dropped the repo into the Clean bucket.
fn commits_in_range(output: &std::process::Output, range: &str) -> Result<bool> {
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git log {range} failed: {}", stderr.trim());
    }
    Ok(!output.stdout.is_empty())
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
