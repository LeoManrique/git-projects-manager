use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
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
