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
    let mut cmd = Command::new("git");
    #[cfg(target_os = "windows")]
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd
}

pub struct GitOperations;

impl GitOperations {
    pub fn is_git_repo(path: &Path) -> bool {
        path.join(".git").is_dir()
    }

    pub fn get_current_branch(repo_path: &Path) -> Result<String> {
        let repo = Repository::open(repo_path)?;
        let head = repo.head()?;

        if let Some(name) = head.shorthand() {
            Ok(name.to_string())
        } else {
            Ok("HEAD".to_string())
        }
    }

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

    pub fn fetch(repo_path: &Path) -> Result<()> {
        git_command()
            .arg("fetch")
            .arg("--quiet")
            .current_dir(repo_path)
            .output()?;

        Ok(())
    }

    pub fn has_unpulled_commits(repo_path: &Path) -> Result<bool> {
        let output = git_command()
            .arg("log")
            .arg("HEAD..@{upstream}")
            .arg("--oneline")
            .current_dir(repo_path)
            .output()?;

        Ok(!output.stdout.is_empty())
    }

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
            anyhow::bail!("Pull failed: {}", error)
        }
    }
}
