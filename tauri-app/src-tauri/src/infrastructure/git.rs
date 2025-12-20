use anyhow::Result;
use git2::{Repository, Status, StatusOptions};
use std::path::Path;
use std::process::Command;

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
        let output = Command::new("git")
            .arg("log")
            .arg("@{upstream}..HEAD")
            .arg("--oneline")
            .current_dir(repo_path)
            .output()?;

        Ok(!output.stdout.is_empty())
    }

    pub fn has_upstream_branch(repo_path: &Path) -> Result<bool> {
        let output = Command::new("git")
            .arg("rev-parse")
            .arg("--abbrev-ref")
            .arg("--symbolic-full-name")
            .arg("@{upstream}")
            .current_dir(repo_path)
            .output()?;

        Ok(output.status.success())
    }
}
