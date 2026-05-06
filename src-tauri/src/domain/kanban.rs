use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KanbanCard {
    pub repo_path: String,
    pub column: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KanbanState {
    pub version: u32,
    pub cards: HashMap<String, KanbanCard>,
}

impl Default for KanbanState {
    fn default() -> Self {
        Self {
            version: 1,
            cards: HashMap::new(),
        }
    }
}
