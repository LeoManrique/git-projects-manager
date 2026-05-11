use crate::domain::auth::SyncStatus;
use crate::infrastructure::github_cli::GhRepo;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

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
    pub fn new(config_dir: &PathBuf) -> Self {
        Self {
            path: config_dir.join("repos_cache_v1.json"),
        }
    }

    pub fn load(&self) -> Result<Option<ReposCache>> {
        if !self.path.exists() {
            return Ok(None);
        }
        let content = fs::read_to_string(&self.path)?;
        Ok(Some(serde_json::from_str(&content)?))
    }

    pub fn save(&self, cache: &ReposCache) -> Result<()> {
        let content = serde_json::to_string_pretty(cache)?;
        fs::write(&self.path, content)?;
        Ok(())
    }
}
