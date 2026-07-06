use crate::domain::RepoStatus;
use crate::infrastructure::git::GitOperations;
use std::path::Path;

/// Responsible for checking the status of a single git repository
pub struct StatusChecker;

impl StatusChecker {
    /// Check the status of a repository at the given path
    pub fn check(path: &Path, only_local_checks: bool) -> RepoStatus {
        let path_str = path.display().to_string();

        // Get branch - handle UnbornBranch (no commits yet) specially
        let (branch, is_unborn) = match GitOperations::get_current_branch(path) {
            Ok(b) => (Some(b), false),
            Err(e) => {
                // UnbornBranch means repo is initialized but has no commits yet
                if GitOperations::is_unborn_branch_error(&e) {
                    (None, true)
                } else {
                    return RepoStatus {
                        path: path_str,
                        branch: None,
                        has_changes: None,
                        has_unpushed: None,
                        has_unpulled: None,
                        has_error: true,
                        error_message: Some(format!("Failed to get branch: {e}")),
                    };
                }
            }
        };

        // Check for pending changes (works for both normal and unborn repos)
        let has_changes = match GitOperations::has_pending_changes(path) {
            Ok(c) => Some(c),
            Err(e) => {
                return RepoStatus {
                    path: path_str,
                    branch,
                    has_changes: None,
                    has_unpushed: None,
                    has_unpulled: None,
                    has_error: true,
                    error_message: Some(format!("Failed to check changes: {e}")),
                };
            }
        };

        // For unborn repos with no changes, mark as unpushed
        let has_unpushed_for_unborn = if is_unborn && has_changes != Some(true) {
            Some(true)
        } else {
            None
        };

        // Check for unpushed/unpulled commits (skip if only_local_checks is enabled)
        let (has_unpushed, has_unpulled) = if only_local_checks {
            (None, None)
        } else {
            Self::check_remote_status(path)
        };

        RepoStatus {
            path: path_str,
            branch,
            has_changes,
            has_unpushed: has_unpushed_for_unborn.or(has_unpushed),
            has_unpulled,
            has_error: false,
            error_message: None,
        }
    }

    /// Check unpushed/unpulled status against remote
    fn check_remote_status(path: &Path) -> (Option<bool>, Option<bool>) {
        if !GitOperations::has_upstream_branch(path).unwrap_or(false) {
            return (None, None);
        }

        // Fetch from remote to get latest state
        let _ = GitOperations::fetch(path);

        let unpushed = GitOperations::has_unpushed_commits(path).ok();
        let unpulled = GitOperations::has_unpulled_commits(path).ok();

        (unpushed, unpulled)
    }
}
