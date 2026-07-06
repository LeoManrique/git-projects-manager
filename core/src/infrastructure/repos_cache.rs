use crate::domain::auth::SyncStatus;
use crate::infrastructure::github_cli::GhRepo;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize)]
pub struct ReposCache {
    pub repos: Vec<GhRepo>,
    pub sync_status: SyncStatus,
    pub fetched_at: i64,
}

pub struct ReposCacheManager {
    path: PathBuf,
}

impl ReposCacheManager {
    #[must_use]
    pub fn new(config_dir: &Path) -> Self {
        Self {
            path: config_dir.join("repos_cache_v1.json"),
        }
    }

    /// # Errors
    /// Returns an error if the cache file cannot be read or parsed.
    pub fn load(&self) -> Result<Option<ReposCache>> {
        if !self.path.exists() {
            return Ok(None);
        }
        let content = fs::read_to_string(&self.path)?;
        Ok(Some(serde_json::from_str(&content)?))
    }

    /// # Errors
    /// Returns an error if serialization or the atomic write to disk fails.
    pub fn save(&self, cache: &ReposCache) -> Result<()> {
        let content = serde_json::to_string_pretty(cache)?;
        crate::infrastructure::atomic_write::write_atomic(&self.path, &content)?;
        Ok(())
    }
}
