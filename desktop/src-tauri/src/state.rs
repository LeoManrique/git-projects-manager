use crate::domain::auth::SyncSession;
use crate::domain::scanner::Scanner;
use crate::infrastructure::{
    config_store::ConfigManager, kanban_store::KanbanManager, repos_cache::ReposCacheManager,
    settings_store::SettingsManager, sync_client::SyncClient,
    token_store::{self, TokenStore},
};
use parking_lot::RwLock;
use std::sync::Arc;

pub struct AppState {
    pub config_manager: Arc<ConfigManager>,
    pub settings_manager: Arc<SettingsManager>,
    pub kanban_manager: Arc<KanbanManager>,
    pub repos_cache: Arc<ReposCacheManager>,
    pub scanner: Arc<RwLock<Scanner>>,
    pub auth: Arc<RwLock<Option<SyncSession>>>,
    pub token_store: Arc<dyn TokenStore>,
    pub sync_client: Arc<SyncClient>,
}

impl AppState {
    pub fn new() -> anyhow::Result<Self> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not find config directory"))?
            .join("git-projects-manager");

        std::fs::create_dir_all(&config_dir)?;

        let token_store = token_store::default_store();
        let auth = match token_store.load() {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!(?e, "failed to load stored sync session");
                None
            }
        };

        Ok(Self {
            config_manager: Arc::new(ConfigManager::new()?),
            settings_manager: Arc::new(SettingsManager::new()?),
            kanban_manager: Arc::new(KanbanManager::new(&config_dir)),
            repos_cache: Arc::new(ReposCacheManager::new(&config_dir)),
            scanner: Arc::new(RwLock::new(Scanner::new())),
            auth: Arc::new(RwLock::new(auth)),
            token_store,
            sync_client: Arc::new(SyncClient::new()?),
        })
    }
}
