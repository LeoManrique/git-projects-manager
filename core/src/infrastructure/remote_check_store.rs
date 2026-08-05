use crate::infrastructure::atomic_write::write_atomic;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// One cached remote-existence verdict, keyed by repo absolute path.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct RemoteCheckEntry {
    /// Unix epoch seconds when the verdict was recorded.
    pub checked_at: i64,
    /// Whether the remote existed at that time.
    pub exists: bool,
}

/// Persisted debounce cache for expensive `gh` remote-existence checks. When
/// git already reports a remote as gone, the (throttled) rescans would re-ask
/// `gh` every time; this caps that to once per TTL by remembering the verdict.
pub struct RemoteCheckStore {
    path: PathBuf,
}

impl Default for RemoteCheckStore {
    fn default() -> Self {
        Self::new()
    }
}

impl RemoteCheckStore {
    /// Self-locating like the other stores
    /// (`dirs::config_dir()/git-projects-manager/remote_checks_v1.json`).
    #[must_use]
    pub fn new() -> Self {
        let path = dirs::config_dir().map_or_else(
            || PathBuf::from("remote_checks_v1.json"),
            |d| {
                d.join("git-projects-manager")
                    .join("remote_checks_v1.json")
            },
        );
        Self { path }
    }

    /// Load the cache, returning an empty map when the file is missing or
    /// unreadable — the cache is advisory, so a miss just forces a re-check.
    #[must_use]
    pub fn load(&self) -> HashMap<String, RemoteCheckEntry> {
        fs::read_to_string(&self.path)
            .ok()
            .and_then(|c| serde_json::from_str(&c).ok())
            .unwrap_or_default()
    }

    /// # Errors
    /// Returns an error if the parent directory cannot be created, or if
    /// serialization or the atomic write to disk fails.
    pub fn save(&self, entries: &HashMap<String, RemoteCheckEntry>) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(entries)?;
        write_atomic(&self.path, &content)?;
        Ok(())
    }
}
