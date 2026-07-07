use crate::domain::kanban::{KanbanCard, KanbanState};
use anyhow::Result;
use parking_lot::Mutex;
use std::fs;
use std::path::{Path, PathBuf};

pub struct KanbanManager {
    path: PathBuf,
    /// Serializes read-modify-write cycles within this process (a card move
    /// racing a repo reconciliation must not lose the move). Cross-process
    /// writers remain last-writer-wins via the atomic rename.
    write_lock: Mutex<()>,
}

impl KanbanManager {
    #[must_use]
    pub fn new(config_dir: &Path) -> Self {
        // v1 lived in kanban.json; v2 starts fresh under a new filename.
        // Best-effort cleanup of the legacy file so it doesn't linger.
        let legacy = config_dir.join("kanban.json");
        if legacy.exists() {
            let _ = fs::remove_file(&legacy);
        }
        Self {
            path: config_dir.join("kanban_v2.json"),
            write_lock: Mutex::new(()),
        }
    }

    /// # Errors
    /// Returns an error if `kanban_v2.json` cannot be read or parsed.
    pub fn load(&self) -> Result<KanbanState> {
        if !self.path.exists() {
            return Ok(KanbanState::default());
        }
        let content = fs::read_to_string(&self.path)?;
        let state: KanbanState = serde_json::from_str(&content)?;
        Ok(state)
    }

    /// Transactionally load the current state, apply `mutate`, and persist —
    /// all under the write lock, so concurrent read-modify-write cycles in
    /// this process cannot lose each other's changes.
    ///
    /// # Errors
    /// Returns an error if the kanban state cannot be loaded or saved.
    pub fn update(&self, mutate: impl FnOnce(&mut KanbanState)) -> Result<KanbanState> {
        let _guard = self.write_lock.lock();
        let mut state = self.load()?;
        mutate(&mut state);
        self.write(&state)?;
        Ok(state)
    }

    /// # Errors
    /// Returns an error if the kanban state cannot be loaded or saved.
    pub fn move_card(&self, name_with_owner: &str, to_column: &str) -> Result<KanbanState> {
        self.update(|state| {
            if let Some(card) = state.cards.get_mut(name_with_owner) {
                card.column = to_column.to_string();
                card.updated_at = chrono::Utc::now().timestamp_millis();
            }
        })
    }

    /// # Errors
    /// Returns an error if the kanban state cannot be loaded or saved.
    pub fn sync_with_repos(&self, names_with_owner: Vec<String>) -> Result<KanbanState> {
        self.update(|state| {
            let now = chrono::Utc::now().timestamp_millis();
            let incoming: std::collections::HashSet<String> =
                names_with_owner.iter().cloned().collect();

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
        })
    }

    fn write(&self, state: &KanbanState) -> Result<()> {
        let content = serde_json::to_string_pretty(state)?;
        crate::infrastructure::atomic_write::write_atomic(&self.path, &content)?;
        Ok(())
    }
}
