use crate::infrastructure::process::output_with_timeout;
use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::LazyLock;
use std::time::Duration;

/// Wall-clock bound on a `gh` call. `gh` has no equivalent of git's transfer
/// timeouts, so an unresponsive api.github.com used to hold a scan thread
/// indefinitely.
const GH_TIMEOUT: Duration = Duration::from_secs(15);
/// `gh repo list` pages through up to 1000 repos, so it gets a longer leash.
const GH_LIST_TIMEOUT: Duration = Duration::from_mins(1);

/// Environment a GUI-launched app cannot see but `gh` may depend on. Captured
/// once from the login shell alongside the binary path (see [`GH`]).
const INHERITED_ENV: [&str; 4] = ["GH_TOKEN", "GITHUB_TOKEN", "GH_HOST", "GH_CONFIG_DIR"];

/// Where `gh` lives and what environment it needs, resolved once per process.
struct GhBinary {
    path: PathBuf,
    env: Vec<(String, String)>,
}

/// `gh` used to be invoked as `$SHELL -lc "gh …"`. A login shell is how a GUI
/// app finds `gh` at all — launched from Finder it inherits none of the user's
/// PATH — but it re-sourced the whole login profile on *every* call: 70 ms
/// measured here against 19 ms for a direct exec. Doing it once keeps the
/// discovery and drops the per-call tax, and passing argv rather than a shell
/// string removes shell quoting from the picture entirely.
static GH: LazyLock<Option<GhBinary>> = LazyLock::new(resolve_gh);

fn resolve_gh() -> Option<GhBinary> {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
    // One login shell: the binary path, then each variable gh might need. The
    // profile can export these, and a direct exec would otherwise lose them.
    let script = format!(
        "command -v gh; {}",
        INHERITED_ENV
            .iter()
            .map(|k| format!("printf '%s\\n' \"${k}\""))
            .collect::<Vec<_>>()
            .join("; ")
    );
    let output = Command::new(shell).arg("-lc").arg(script).output().ok()?;
    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut lines = stdout.lines();
    let path = PathBuf::from(lines.next()?.trim());
    if path.as_os_str().is_empty() {
        return None;
    }

    let env = INHERITED_ENV
        .iter()
        .zip(lines)
        .filter(|(_, v)| !v.trim().is_empty())
        .map(|(k, v)| ((*k).to_string(), v.trim().to_string()))
        .collect();

    Some(GhBinary { path, env })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GhRepo {
    pub name_with_owner: String,
    pub name: String,
    pub owner: GhOwner,
    pub description: Option<String>,
    pub url: String,
    pub is_private: bool,
    pub is_archived: bool,
    pub pushed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GhOwner {
    pub login: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "status")]
pub enum GhAuthStatus {
    Ok { user: String },
    NotInstalled,
    NotAuthenticated,
    Error { message: String },
}

/// A `gh` invocation with the resolved binary and its captured environment, or
/// `None` when `gh` is not installed.
fn gh() -> Option<Command> {
    let binary = GH.as_ref()?;
    let mut cmd = Command::new(&binary.path);
    for (key, value) in &binary.env {
        cmd.env(key, value);
    }
    Some(cmd)
}

/// Run `gh` with the given arguments, returning stdout and stderr combined —
/// `gh` splits its messages across both and every caller wants the whole story.
fn gh_output(args: &[&str], dir: Option<&Path>, timeout: Duration) -> Option<(bool, String)> {
    let mut cmd = gh()?;
    cmd.args(args);
    if let Some(dir) = dir {
        cmd.current_dir(dir);
    }
    let output = output_with_timeout(&mut cmd, timeout).ok()?;
    let combined = String::from_utf8_lossy(&output.stdout).to_string()
        + &String::from_utf8_lossy(&output.stderr);
    Some((output.status.success(), combined))
}

#[must_use]
pub fn check_auth() -> GhAuthStatus {
    let Some((success, combined)) =
        gh_output(&["auth", "status", "--hostname", "github.com"], None, GH_TIMEOUT)
    else {
        // Either the binary was never found, or the call timed out. The former
        // is by far the likelier and is what callers act on.
        return GhAuthStatus::NotInstalled;
    };

    if !success {
        if combined.contains("not logged") || combined.contains("not been authenticated") {
            return GhAuthStatus::NotAuthenticated;
        }
        return GhAuthStatus::Error { message: combined.trim().to_string() };
    }

    let user = combined
        .lines()
        .find_map(|l| {
            l.split_once("account ")
                .map(|(_, rest)| rest.split_whitespace().next().unwrap_or("").to_string())
        })
        .unwrap_or_default();

    GhAuthStatus::Ok { user }
}

/// Whether the GitHub repository a local clone points to still exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepoExistence {
    /// `gh` confirmed the repository exists.
    Exists,
    /// The host reports the repository does not exist.
    NotFound,
    /// Could not determine (gh missing/unauthenticated, offline, or the remote
    /// is not a GitHub host). Callers must treat this as "don't flag".
    Unknown,
}

/// Classify the result of `gh repo view`. Kept pure (no I/O) for unit tests.
///
/// Only an explicit unresolvable-repository message counts as
/// [`RepoExistence::NotFound`]; every other failure (auth, network, non-GitHub
/// remote) is [`RepoExistence::Unknown`] so a transient error never masquerades
/// as a deleted remote.
#[must_use]
pub fn classify_repo_view(success: bool, combined: &str) -> RepoExistence {
    if success {
        return RepoExistence::Exists;
    }
    let s = combined.to_lowercase();

    // The shell, not GitHub, when `gh` is absent from PATH: "command not found:
    // gh". That matched the bare "not found" test this replaces, so a missing
    // CLI flagged every repo as deleted — and the verdict was then cached for a
    // day. `check_auth` has always guarded this; this call site never did.
    if s.contains("command not found") || s.contains("not found: gh") {
        return RepoExistence::Unknown;
    }

    if s.contains("could not resolve to a repository") || s.contains("repository not found") {
        RepoExistence::NotFound
    } else {
        RepoExistence::Unknown
    }
}

/// Ask `gh` whether the GitHub repo behind a local clone still exists, running
/// in the repo's directory so `gh` resolves the remote itself (no URL parsing).
///
/// Returns [`RepoExistence::Unknown`] if `gh` cannot be spawned, so a missing
/// CLI degrades gracefully rather than flagging repos.
#[must_use]
pub fn repo_exists_in_dir(repo_path: &Path) -> RepoExistence {
    let Some((success, combined)) = gh_output(
        &["repo", "view", "--json", "name"],
        Some(repo_path),
        GH_TIMEOUT,
    ) else {
        return RepoExistence::Unknown;
    };
    classify_repo_view(success, &combined)
}

fn validate_name_with_owner(nwo: &str) -> Result<()> {
    let mut slashes = 0;
    for c in nwo.chars() {
        if c == '/' {
            slashes += 1;
        } else if !(c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_')) {
            return Err(anyhow!("invalid repo name: {nwo}"));
        }
    }
    if slashes != 1 || nwo.starts_with('/') || nwo.ends_with('/') {
        return Err(anyhow!("invalid repo name: {nwo}"));
    }
    Ok(())
}

/// # Errors
/// Returns an error if `name_with_owner` is not a valid `owner/repo` name,
/// if the `gh` CLI cannot be spawned, or if `gh repo delete` exits with a
/// failure status.
pub fn delete_repo(name_with_owner: &str) -> Result<()> {
    // The name no longer reaches a shell (argv, not an interpolated string), so
    // this is now a plain input check rather than an injection guard.
    validate_name_with_owner(name_with_owner)?;
    let (success, combined) = gh_output(
        &["repo", "delete", name_with_owner, "--yes"],
        None,
        GH_TIMEOUT,
    )
    .context("failed to run gh")?;
    if !success {
        return Err(anyhow!("gh repo delete failed: {}", combined.trim()));
    }
    Ok(())
}

/// # Errors
/// Returns an error if the `gh` CLI cannot be spawned, if `gh repo list`
/// exits with a failure status, or if its JSON output cannot be parsed.
pub fn list_repos() -> Result<Vec<GhRepo>> {
    let (success, combined) = gh_output(
        &[
            "repo",
            "list",
            "--limit",
            "1000",
            "--json",
            "nameWithOwner,name,owner,description,url,isPrivate,isArchived,pushedAt",
        ],
        None,
        GH_LIST_TIMEOUT,
    )
    .context("failed to run gh")?;

    if !success {
        return Err(anyhow!("gh repo list failed: {}", combined.trim()));
    }

    let repos: Vec<GhRepo> = serde_json::from_str(&combined)
        .with_context(|| format!("failed to parse gh output: {combined}"))?;
    Ok(repos)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repo_view_success_means_exists() {
        assert_eq!(classify_repo_view(true, "{\"name\":\"x\"}"), RepoExistence::Exists);
    }

    #[test]
    fn repo_view_graphql_resolve_error_means_not_found() {
        assert_eq!(
            classify_repo_view(
                false,
                "GraphQL: Could not resolve to a Repository with the name 'owner/repo'. (repository)"
            ),
            RepoExistence::NotFound
        );
    }

    #[test]
    fn repo_view_non_github_or_auth_error_is_unknown() {
        // Remote is not a GitHub host → cannot judge existence.
        assert_eq!(
            classify_repo_view(
                false,
                "none of the git remotes configured for this repository point to a known GitHub host"
            ),
            RepoExistence::Unknown
        );
        // Offline / connectivity.
        assert_eq!(
            classify_repo_view(false, "error connecting to api.github.com"),
            RepoExistence::Unknown
        );
    }

    #[test]
    fn repo_view_missing_gh_binary_is_unknown() {
        // The shell's own error, not GitHub's — must never read as "deleted".
        assert_eq!(
            classify_repo_view(false, "zsh:1: command not found: gh"),
            RepoExistence::Unknown
        );
        assert_eq!(
            classify_repo_view(false, "/bin/sh: 1: gh: not found"),
            RepoExistence::Unknown
        );
        assert_eq!(
            classify_repo_view(false, "bash: line 1: gh: command not found"),
            RepoExistence::Unknown
        );
    }

    #[test]
    fn repo_view_rest_not_found_means_not_found() {
        assert_eq!(
            classify_repo_view(false, "HTTP 404: Repository not found (https://api.github.com/repos/o/r)"),
            RepoExistence::NotFound
        );
    }
}
