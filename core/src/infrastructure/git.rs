use anyhow::Result;
use git2::{Repository, Status, StatusOptions};
use std::path::Path;
use std::process::Command;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

/// Creates a git command with platform-specific settings to hide console windows
fn git_command() -> Command {
    #[allow(unused_mut)]
    let mut cmd = Command::new("git");
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
    /// Returns an error if the `git log` command cannot be executed.
    pub fn has_unpushed_commits(repo_path: &Path) -> Result<bool> {
        // Using git command for simplicity as git2 branch tracking is complex
        let output = git_command()
            .arg("log")
            .arg("@{upstream}..HEAD")
            .arg("--oneline")
            .current_dir(repo_path)
            .output()?;

        Ok(!output.stdout.is_empty())
    }

    /// True if the repository has at least one remote configured. A repo with
    /// no remote has never been published to a host (see `ScanResult.unpublished`).
    ///
    /// # Errors
    /// Returns an error if the `git remote` command cannot be executed.
    pub fn has_remote(repo_path: &Path) -> Result<bool> {
        let output = git_command()
            .arg("remote")
            .current_dir(repo_path)
            .output()?;

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

    /// # Errors
    /// Returns an error if the `git fetch` command cannot be executed.
    pub fn fetch(repo_path: &Path) -> Result<()> {
        git_command()
            .arg("fetch")
            .arg("--quiet")
            .current_dir(repo_path)
            .output()?;

        Ok(())
    }

    /// # Errors
    /// Returns an error if the `git log` command cannot be executed.
    pub fn has_unpulled_commits(repo_path: &Path) -> Result<bool> {
        let output = git_command()
            .arg("log")
            .arg("HEAD..@{upstream}")
            .arg("--oneline")
            .current_dir(repo_path)
            .output()?;

        Ok(!output.stdout.is_empty())
    }

    /// # Errors
    /// Returns an error if the `git fetch`/`git pull` commands cannot be
    /// executed, or if `git pull` exits with a failure status.
    pub fn pull(repo_path: &Path) -> Result<String> {
        // First fetch
        Self::fetch(repo_path)?;

        // Then pull
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
    /// with a failure status, or if deleting a listed file or directory fails.
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

            if is_dir {
                std::fs::remove_dir_all(&abs)?;
                directories_removed.push(clean_path.to_string());
            } else {
                std::fs::remove_file(&abs)?;
                files_removed.push(path.to_string());
            }
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
}
