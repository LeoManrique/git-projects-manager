use serde::{Deserialize, Serialize};

/// Where a repo stands relative to a remote host. Single source of truth for
/// the publish-related overlays (`ScanResult.unpublished` / `.remote_not_found`);
/// adding a future state is a new variant plus one arm in `categorize_results`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PublishState {
    /// A remote is configured and either verified reachable or not checked.
    /// This is the safe default whenever the remote's existence is uncertain
    /// (offline, auth failure, non-GitHub remote, or `only_local_checks`).
    Published,
    /// No remote configured — the repo has never been published to a host.
    Unpublished,
    /// A remote is configured but the host reports it no longer exists
    /// (git fetch says "not found" *and* `gh` confirms it is gone).
    RemoteNotFound,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoStatus {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    pub has_changes: Option<bool>,
    pub has_unpushed: Option<bool>,
    pub has_unpulled: Option<bool>,
    /// Publish state relative to the remote host (see [`PublishState`]).
    pub publish_state: PublishState,
    pub has_error: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanResult {
    pub scanned_path: String,
    pub total_repositories: usize,
    pub with_changes: Vec<RepoStatus>,
    pub with_unpushed: Vec<RepoStatus>,
    pub with_unpulled: Vec<RepoStatus>,
    /// Repos with no remote configured (never published). *Overlay* category:
    /// a repo here also appears in its primary status bucket above.
    pub unpublished: Vec<RepoStatus>,
    /// Repos whose configured remote no longer exists on the host. *Overlay*
    /// category, like `unpublished` — the repo also appears in its primary bucket.
    pub remote_not_found: Vec<RepoStatus>,
    pub clean: Vec<RepoStatus>,
    pub errors: Vec<RepoStatus>,
    pub uninitialized: Vec<RepoStatus>,
    pub execution_time: f64,
}
