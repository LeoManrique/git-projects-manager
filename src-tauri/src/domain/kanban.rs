use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

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

pub struct KanbanManager {
    path: PathBuf,
}

impl KanbanManager {
    pub fn new(config_dir: &PathBuf) -> Self {
        Self {
            path: config_dir.join("kanban.json"),
        }
    }

    pub fn load(&self) -> Result<KanbanState> {
        if !self.path.exists() {
            return Ok(KanbanState::default());
        }
        let content = fs::read_to_string(&self.path)?;
        let state: KanbanState = serde_json::from_str(&content)?;
        Ok(state)
    }

    pub fn save(&self, state: &KanbanState) -> Result<()> {
        let content = serde_json::to_string_pretty(state)?;
        fs::write(&self.path, content)?;
        Ok(())
    }

    pub fn move_card(&self, repo_path: &str, to_column: &str) -> Result<KanbanState> {
        let mut state = self.load()?;
        if let Some(card) = state.cards.get_mut(repo_path) {
            card.column = to_column.to_string();
            card.updated_at = chrono::Utc::now().timestamp_millis();
        }
        self.save(&state)?;
        Ok(state)
    }

    pub fn update_notes(&self, repo_path: &str, notes: Option<String>) -> Result<KanbanState> {
        let mut state = self.load()?;
        if let Some(card) = state.cards.get_mut(repo_path) {
            card.notes = notes;
            card.updated_at = chrono::Utc::now().timestamp_millis();
        }
        self.save(&state)?;
        Ok(state)
    }

    pub fn remove_card(&self, repo_path: &str) -> Result<KanbanState> {
        let mut state = self.load()?;
        state.cards.remove(repo_path);
        self.save(&state)?;
        Ok(state)
    }

    pub fn sync_with_repos(&self, repo_paths: Vec<String>) -> Result<KanbanState> {
        let mut state = self.load()?;
        let now = chrono::Utc::now().timestamp_millis();

        for path in repo_paths {
            if !state.cards.contains_key(&path) {
                state.cards.insert(
                    path.clone(),
                    KanbanCard {
                        repo_path: path,
                        column: "backlog".to_string(),
                        notes: None,
                        created_at: now,
                        updated_at: now,
                    },
                );
            }
        }

        self.save(&state)?;
        Ok(state)
    }
}
