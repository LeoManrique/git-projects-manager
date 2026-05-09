use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoStatus {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    pub has_changes: Option<bool>,
    pub has_unpushed: Option<bool>,
    pub has_unpulled: Option<bool>,
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
    pub clean: Vec<RepoStatus>,
    pub errors: Vec<RepoStatus>,
    pub uninitialized: Vec<RepoStatus>,
    pub execution_time: f64,
}
