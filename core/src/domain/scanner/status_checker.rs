use super::remote_check::RemoteCheckCtx;
use crate::domain::{PublishState, RepoStatus};
use crate::infrastructure::git::{GitOperations, RemoteReachability};
use std::path::Path;

/// Responsible for checking the status of a single git repository
pub struct StatusChecker;

impl StatusChecker {
    /// Check the status of a repository at the given path.
    ///
    /// `remote_ctx` is present only for online scans; it debounces the `gh`
    /// confirmation used to promote a repo to [`PublishState::RemoteNotFound`].
    pub fn check(
        path: &Path,
        only_local_checks: bool,
        remote_ctx: Option<&RemoteCheckCtx>,
    ) -> RepoStatus {
        let path_str = path.display().to_string();

        // Whether the repo was ever published (has a remote). Purely local, so
        // it runs even when only_local_checks skips network round-trips.
        let has_remote = GitOperations::has_remote(path).ok();

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
                        publish_state: Self::base_publish_state(has_remote),
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
                    publish_state: Self::base_publish_state(has_remote),
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
        let (has_unpushed, has_unpulled, reachability) = if only_local_checks {
            (None, None, None)
        } else {
            Self::check_remote_status(path)
        };

        let publish_state =
            Self::determine_publish_state(path, has_remote, reachability, remote_ctx);

        RepoStatus {
            path: path_str,
            branch,
            has_changes,
            has_unpushed: has_unpushed_for_unborn.or(has_unpushed),
            has_unpulled,
            publish_state,
            has_error: false,
            error_message: None,
        }
    }

    /// Publish state from local signal alone: no remote → Unpublished, otherwise
    /// Published. Used for error paths and as the baseline before the (online-
    /// only) `RemoteNotFound` promotion.
    fn base_publish_state(has_remote: Option<bool>) -> PublishState {
        match has_remote {
            Some(false) => PublishState::Unpublished,
            _ => PublishState::Published,
        }
    }

    /// Promote a published repo to `RemoteNotFound` only when `git fetch`
    /// definitively said "not found" *and* the debounced `gh` check confirms it.
    /// Every uncertain case falls back to the local-only baseline.
    fn determine_publish_state(
        path: &Path,
        has_remote: Option<bool>,
        reachability: Option<RemoteReachability>,
        remote_ctx: Option<&RemoteCheckCtx>,
    ) -> PublishState {
        let base = Self::base_publish_state(has_remote);
        if base == PublishState::Published
            && has_remote == Some(true)
            && reachability == Some(RemoteReachability::NotFound)
            && let Some(ctx) = remote_ctx
            && ctx.is_remote_gone(path)
        {
            return PublishState::RemoteNotFound;
        }
        base
    }

    /// Check unpushed/unpulled status against remote, returning the fetch's
    /// reachability so the caller can detect a deleted remote. Only repos with
    /// an upstream branch are probed (others yield `None` on every field).
    fn check_remote_status(
        path: &Path,
    ) -> (Option<bool>, Option<bool>, Option<RemoteReachability>) {
        if !GitOperations::has_upstream_branch(path).unwrap_or(false) {
            return (None, None, None);
        }

        // Fetch from remote to get latest state (and classify reachability).
        let reachability = GitOperations::fetch(path).ok();

        let unpushed = GitOperations::has_unpushed_commits(path).ok();
        let unpulled = GitOperations::has_unpulled_commits(path).ok();

        (unpushed, unpulled, reachability)
    }
}
