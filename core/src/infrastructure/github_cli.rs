use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;

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

fn gh() -> Command {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
    let mut cmd = Command::new(shell);
    cmd.arg("-lc");
    cmd
}

#[must_use]
pub fn check_auth() -> GhAuthStatus {
    let output = gh().arg("gh auth status --hostname github.com 2>&1").output();

    let output = match output {
        Ok(o) => o,
        Err(e) => return GhAuthStatus::Error { message: e.to_string() },
    };

    let combined = String::from_utf8_lossy(&output.stdout).to_string()
        + &String::from_utf8_lossy(&output.stderr);

    if combined.contains("command not found") || combined.contains("not found: gh") {
        return GhAuthStatus::NotInstalled;
    }

    if !output.status.success() {
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
/// Only an explicit "not found" / unresolvable-repository message counts as
/// [`RepoExistence::NotFound`]; every other failure (auth, network, non-GitHub
/// remote) is [`RepoExistence::Unknown`] so a transient error never masquerades
/// as a deleted remote.
#[must_use]
pub fn classify_repo_view(success: bool, combined: &str) -> RepoExistence {
    if success {
        return RepoExistence::Exists;
    }
    let s = combined.to_lowercase();
    if s.contains("could not resolve to a repository") || s.contains("not found") {
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
    let output = gh()
        .arg("gh repo view --json name")
        .current_dir(repo_path)
        .output();

    let Ok(output) = output else {
        return RepoExistence::Unknown;
    };

    let combined = String::from_utf8_lossy(&output.stdout).to_string()
        + &String::from_utf8_lossy(&output.stderr);
    classify_repo_view(output.status.success(), &combined)
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
    validate_name_with_owner(name_with_owner)?;
    let cmd = format!("gh repo delete {name_with_owner} --yes");
    let output = gh().arg(cmd).output().context("failed to spawn gh")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let msg = format!("{}{}", stderr.trim(), stdout.trim());
        return Err(anyhow!("gh repo delete failed: {msg}"));
    }
    Ok(())
}

/// # Errors
/// Returns an error if the `gh` CLI cannot be spawned, if `gh repo list`
/// exits with a failure status, or if its JSON output cannot be parsed.
pub fn list_repos() -> Result<Vec<GhRepo>> {
    let output = gh()
        .arg("gh repo list --limit 1000 --json nameWithOwner,name,owner,description,url,isPrivate,isArchived,pushedAt")
        .output()
        .context("failed to spawn gh")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("gh repo list failed: {}", stderr.trim()));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let repos: Vec<GhRepo> = serde_json::from_str(&stdout)
        .with_context(|| format!("failed to parse gh output: {stdout}"))?;
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
}
