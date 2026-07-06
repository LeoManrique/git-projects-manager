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
    #[must_use]
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
