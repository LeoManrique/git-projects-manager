pub mod config;
pub mod kanban;
pub mod repository;
pub mod scanner;
pub mod settings;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitoredFolder {
    pub id: String,
    pub path: String,
    pub name: String,
    #[serde(default)]
    pub only_local_checks: bool,
}

impl MonitoredFolder {
    pub fn new(path: String, name: String, only_local_checks: bool) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            path,
            name,
            only_local_checks,
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

// Terminal app configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalApp {
    pub id: String,
    pub name: String,
    pub display_name: String,
    pub path: String,
    #[serde(default)]
    pub open_method: OpenMethod,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum OpenMethod {
    #[default]
    Default,
    #[serde(rename = "open")]
    Open,
    #[serde(rename = "applescript")]
    AppleScript { script: String },
}

// Editor app configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorApp {
    pub id: String,
    pub name: String,
    pub display_name: String,
    pub path: String,
}

// Git clean settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitCleanSettings {
    #[serde(default)]
    pub exclude_patterns: Vec<String>,
}

impl Default for GitCleanSettings {
    fn default() -> Self {
        Self {
            exclude_patterns: vec![
                ".env*".to_string(),
                "*.key".to_string(),
                "*.pem".to_string(),
                ".vscode/".to_string(),
                ".idea/".to_string(),
            ],
        }
    }
}

// Git clean result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitCleanResult {
    pub files_removed: Vec<String>,
    pub directories_removed: Vec<String>,
}

// App settings
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    #[serde(default)]
    pub default_terminal: Option<String>,
    #[serde(default)]
    pub default_editor: Option<String>,
    #[serde(default)]
    pub git_clean_settings: Option<GitCleanSettings>,
}
