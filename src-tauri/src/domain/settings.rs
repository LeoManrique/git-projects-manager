use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorApp {
    pub id: String,
    pub name: String,
    pub display_name: String,
    pub path: String,
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitCleanResult {
    pub files_removed: Vec<String>,
    pub directories_removed: Vec<String>,
}

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
