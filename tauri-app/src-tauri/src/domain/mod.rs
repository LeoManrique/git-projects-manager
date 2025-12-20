pub mod config;
pub mod repository;
pub mod scanner;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitoredFolder {
    pub id: String,
    pub path: String,
    pub name: String,
}

impl MonitoredFolder {
    pub fn new(path: String, name: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            path,
            name,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub folders: Vec<MonitoredFolder>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoStatus {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    pub has_changes: Option<bool>,
    pub has_unpushed: Option<bool>,
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
    pub clean: Vec<RepoStatus>,
    pub errors: Vec<RepoStatus>,
    pub execution_time: f64,
}
