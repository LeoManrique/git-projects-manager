use crate::infrastructure::atomic_write::write_atomic;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Serializes the read-merge-write in [`RemoteCheckStore::save`].
///
/// Both frontends scan their monitored folders concurrently, and each folder
/// scan persists the whole map. `write_atomic` prevents a *corrupt* file, not a
/// *lost update*: the later writer replaced the earlier one's verdicts
/// wholesale, so the 24 h debounce never held and `gh` re-ran every scan.
static SAVE_LOCK: Mutex<()> = Mutex::new(());

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

    /// Load the cache with entries for paths that no longer exist dropped,
    /// reporting whether anything was dropped.
    ///
    /// Nothing ever removed an entry, so the file only grew: a repo deleted
    /// from disk kept its verdict forever. Five of the six entries in a real
    /// user's cache were dead paths.
    #[must_use]
    pub fn load_pruned(&self) -> (HashMap<String, RemoteCheckEntry>, bool) {
        let mut map = self.load();
        let before = map.len();
        map.retain(|path, _| Path::new(path).is_dir());
        let pruned = map.len() != before;
        (map, pruned)
    }

    /// Merge `entries` into whatever is on disk and write the result.
    ///
    /// Read-merge-write rather than overwrite, because two folder scans (and
    /// the two apps) persist concurrently. Conflicts resolve to the newer
    /// `checked_at`, so a stale in-memory snapshot cannot undo a fresher
    /// verdict written by someone else.
    ///
    /// # Errors
    /// Returns an error if the parent directory cannot be created, or if
    /// serialization or the atomic write to disk fails.
    pub fn save(&self, entries: &HashMap<String, RemoteCheckEntry>) -> Result<()> {
        let _guard = SAVE_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut merged = self.load();
        for (path, entry) in entries {
            merged
                .entry(path.clone())
                .and_modify(|existing| {
                    if entry.checked_at >= existing.checked_at {
                        *existing = *entry;
                    }
                })
                .or_insert(*entry);
        }
        merged.retain(|path, _| Path::new(path).is_dir());

        let content = serde_json::to_string_pretty(&merged)?;
        write_atomic(&self.path, &content)?;
        Ok(())
    }
}
