use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KanbanCard {
    pub name_with_owner: String,
    pub column: String,
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
            version: 2,
            cards: HashMap::new(),
        }
    }
}
