use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KanbanCard {
    pub name_with_owner: String,
    pub column: String,
    /// Free text the user keeps on the card ("what to do next"). Omitted
    /// from the JSON when empty, so a file written before notes existed
    /// still loads and a card without notes serializes as it always did.
    #[serde(default, skip_serializing_if = "Option::is_none")]
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
            version: 2,
            cards: HashMap::new(),
        }
    }
}

/// The single rule for stored notes: surrounding whitespace is dropped, and
/// text that is empty after that means "no notes", never `Some("")`.
#[must_use]
pub fn normalize_notes(input: Option<String>) -> Option<String> {
    input
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
}
