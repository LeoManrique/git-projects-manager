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
    /// The remote comparison was attempted and failed, so `has_unpushed` and
    /// `has_unpulled` are unknown rather than false.
    ///
    /// Distinct from "not attempted": under `only_local_checks` the scan never
    /// asks about the remote, and that is a setting working as intended, not a
    /// failure — this stays `false` there. Without the distinction both cases
    /// are `None`, and `None` falls through to the Clean bucket, so a repo with
    /// unpushed work reported as clean was indistinguishable from a real one.
    pub remote_state_unknown: bool,
    /// Publish state relative to the remote host (see [`PublishState`]).
    pub publish_state: PublishState,
    pub has_error: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

impl RepoStatus {
    /// A repo known only by its path: found on disk, nothing checked yet.
    #[must_use]
    pub fn unchecked(path: String) -> Self {
        Self {
            path,
            branch: None,
            has_changes: None,
            has_unpushed: None,
            has_unpulled: None,
            remote_state_unknown: false,
            publish_state: PublishState::Published,
            has_error: false,
            error_message: None,
        }
    }
}

/// One folder's state at one moment. A scan emits several of these as its
/// repos are checked; the last one has `is_complete` set.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanResult {
    pub scanned_path: String,
    /// Repos found by the latest walk, `checking` included.
    pub total_repositories: usize,
    /// Wall clock at the start of the scan this snapshot belongs to, unix ms.
    pub started_at_ms: i64,
    /// Increases with every change to any folder's state, so a frontend can
    /// drop a snapshot that arrives after a newer one.
    pub revision: u64,
    /// Repos the scan in flight has not checked yet. Repos with a previous
    /// status keep it in their bucket meanwhile; the rest are in `checking`.
    pub pending: Vec<String>,
    /// Repos found by this scan that have no status yet (path only). An
    /// exclusive bucket, like `clean`.
    pub checking: Vec<RepoStatus>,
    /// True once the scan has checked every repo it found.
    pub is_complete: bool,
    pub with_changes: Vec<RepoStatus>,
    pub with_unpushed: Vec<RepoStatus>,
    pub with_unpulled: Vec<RepoStatus>,
    /// Repos with no remote configured (never published). *Overlay* category:
    /// a repo here also appears in its primary status bucket above.
    pub unpublished: Vec<RepoStatus>,
    /// Repos whose configured remote no longer exists on the host. *Overlay*
    /// category, like `unpublished` — the repo also appears in its primary bucket.
    pub remote_not_found: Vec<RepoStatus>,
    /// Repos whose remote comparison was attempted and failed. *Overlay*
    /// category: the repo keeps its primary bucket (usually Clean, since an
    /// unknown count cannot put it anywhere else) and carries this on top,
    /// which is the whole point — the bucket alone was asserting something the
    /// scan never established. Empty for `only_local_checks` folders.
    pub remote_state_unknown: Vec<RepoStatus>,
    pub clean: Vec<RepoStatus>,
    pub errors: Vec<RepoStatus>,
    pub uninitialized: Vec<RepoStatus>,
    /// Seconds the scan took. Zero until `is_complete`.
    pub execution_time: f64,
}
