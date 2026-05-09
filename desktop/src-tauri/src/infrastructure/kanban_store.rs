use crate::domain::kanban::{KanbanCard, KanbanState};
use anyhow::Result;
use std::fs;
use std::path::PathBuf;

pub struct KanbanManager {
    path: PathBuf,
}

impl KanbanManager {
    pub fn new(config_dir: &PathBuf) -> Self {
        // v1 lived in kanban.json; v2 starts fresh under a new filename.
        // Best-effort cleanup of the legacy file so it doesn't linger.
        let legacy = config_dir.join("kanban.json");
        if legacy.exists() {
            let _ = fs::remove_file(&legacy);
        }
        Self {
            path: config_dir.join("kanban_v2.json"),
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

    pub fn move_card(&self, name_with_owner: &str, to_column: &str) -> Result<KanbanState> {
        let mut state = self.load()?;
        if let Some(card) = state.cards.get_mut(name_with_owner) {
            card.column = to_column.to_string();
            card.updated_at = chrono::Utc::now().timestamp_millis();
        }
        self.save(&state)?;
        Ok(state)
    }

    pub fn sync_with_repos(&self, names_with_owner: Vec<String>) -> Result<KanbanState> {
        let mut state = self.load()?;
        let now = chrono::Utc::now().timestamp_millis();

        let incoming: std::collections::HashSet<String> = names_with_owner.iter().cloned().collect();

        // Drop cards for repos that no longer exist on GitHub.
        state.cards.retain(|key, _| incoming.contains(key));

        // Insert cards for new repos in the backlog.
        for nwo in names_with_owner {
            state.cards.entry(nwo.clone()).or_insert(KanbanCard {
                name_with_owner: nwo,
                column: "backlog".to_string(),
                created_at: now,
                updated_at: now,
            });
        }

        self.save(&state)?;
        Ok(state)
    }
}
