use super::remote_check::RemoteCheckCtx;
use crate::domain::{PublishState, RepoStatus};
use crate::infrastructure::git::{GitOperations, RemoteReachability, RepoInspector};
use crate::infrastructure::logging::log_safe;
use std::path::Path;

/// Outcome of the remote half of a status check.
///
/// The default is "we did not ask" — no upstream, or `only_local_checks` — which
/// is deliberately *not* the same as `unknown`.
#[derive(Default)]
struct RemoteStatus {
    has_unpushed: Option<bool>,
    has_unpulled: Option<bool>,
    reachability: Option<RemoteReachability>,
    /// We asked and could not get an answer.
    unknown: bool,
}

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

        // One handle for every local check on this repo. If it cannot even be
        // opened there is nothing to report but the failure.
        let repo = match RepoInspector::open(path) {
            Ok(r) => r,
            Err(e) => {
                return Self::failed(
                    path_str,
                    None,
                    PublishState::Published,
                    format!("Failed to open repository: {e}"),
                );
            }
        };

        // Whether the repo was ever published (has a remote). Purely local, so
        // it runs even when only_local_checks skips network round-trips.
        let has_remote = repo.has_remote().ok();

        // Get branch - handle UnbornBranch (no commits yet) specially
        let (branch, is_unborn) = match repo.current_branch() {
            Ok(b) => (Some(b), false),
            Err(e) => {
                // UnbornBranch means repo is initialized but has no commits yet
                if GitOperations::is_unborn_branch_error(&e) {
                    (None, true)
                } else {
                    return Self::failed(
                        path_str,
                        None,
                        Self::base_publish_state(has_remote),
                        format!("Failed to get branch: {e}"),
                    );
                }
            }
        };

        // Check for pending changes (works for both normal and unborn repos)
        let has_changes = match repo.has_pending_changes() {
            Ok(c) => Some(c),
            Err(e) => {
                return Self::failed(
                    path_str,
                    branch,
                    Self::base_publish_state(has_remote),
                    format!("Failed to check changes: {e}"),
                );
            }
        };

        // For unborn repos with no changes, mark as unpushed
        let has_unpushed_for_unborn = if is_unborn && has_changes != Some(true) {
            Some(true)
        } else {
            None
        };

        // Check for unpushed/unpulled commits (skip if only_local_checks is enabled)
        let remote = if only_local_checks {
            // Not attempted, so not unknown — the folder is configured this way.
            RemoteStatus::default()
        } else {
            Self::check_remote_status(path, &repo)
        };
        let RemoteStatus { has_unpushed, has_unpulled, reachability, unknown } = remote;

        let publish_state =
            Self::determine_publish_state(path, has_remote, reachability, remote_ctx);

        RepoStatus {
            path: path_str,
            branch,
            has_changes,
            has_unpushed: has_unpushed_for_unborn.or(has_unpushed),
            has_unpulled,
            remote_state_unknown: unknown,
            publish_state,
            has_error: false,
            error_message: None,
        }
    }

    /// The status of a repo whose local check failed, which lands it in the
    /// Errors bucket. Logged, because the row shows only the first line of
    /// `message`.
    fn failed(
        path: String,
        branch: Option<String>,
        publish_state: PublishState,
        message: String,
    ) -> RepoStatus {
        tracing::warn!(repo = %path, error = %log_safe(&message), "repo check failed");
        RepoStatus {
            path,
            branch,
            has_changes: None,
            has_unpushed: None,
            has_unpulled: None,
            remote_state_unknown: false,
            publish_state,
            has_error: true,
            error_message: Some(message),
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
    fn check_remote_status(path: &Path, repo: &RepoInspector) -> RemoteStatus {
        let Some(upstream) = repo.upstream_ref() else {
            // No upstream to compare against. Nothing failed — there is simply
            // no question to answer — so this is not "unknown".
            return RemoteStatus::default();
        };

        // Fetch from remote to get latest state (and classify reachability).
        let reachability = GitOperations::fetch(path).ok();

        // Re-opened rather than reused. Refs and objects *are* re-read per
        // lookup (verified), but a few things load once when the repository is
        // opened — notably the shallow-clone boundary in `.git/shallow`, which
        // the fetch above can rewrite. Defensive rather than a fixed bug: no
        // failing case could be constructed, because the merge base between a
        // branch and its own upstream is recent and a shallow clone has no deep
        // local history to walk past. Kept because an open costs ~60 µs against
        // a failure mode that is a silent miscategorization.
        let reopened = RepoInspector::open(path);
        let repo = reopened.as_ref().unwrap_or(repo);

        // Deliberately after the fetch: the tracking ref it just advanced is
        // exactly what `ahead_behind` reads.
        let counts = repo
            .ahead_behind(&upstream)
            // libgit2 cannot answer for every repo (replace refs, a
            // commit-graph over a shallow boundary, a non-UTF-8 refname). Ask
            // git itself rather than reporting "no commits either way", which
            // reads as Clean.
            .or_else(|_| GitOperations::ahead_behind_via_cli(path));

        match counts {
            Ok((unpushed, unpulled)) => RemoteStatus {
                has_unpushed: Some(unpushed),
                has_unpulled: Some(unpulled),
                reachability,
                unknown: false,
            },
            // Both git and libgit2 declined to answer. Say so instead of
            // letting two `None`s read as "nothing to push or pull".
            Err(e) => {
                tracing::warn!(
                    repo = %path.display(),
                    error = %log_safe(&format!("{e:#}")),
                    "ahead/behind unknown"
                );
                RemoteStatus {
                    reachability,
                    unknown: true,
                    ..RemoteStatus::default()
                }
            }
        }
    }
}
