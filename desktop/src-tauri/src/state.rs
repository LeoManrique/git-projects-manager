use crate::domain::scanner::Scanner;
use crate::infrastructure::{
    config_store::ConfigManager, kanban_store::KanbanManager, settings_store::SettingsManager,
};
use parking_lot::RwLock;
use std::sync::Arc;

pub struct AppState {
    pub config_manager: Arc<ConfigManager>,
    pub settings_manager: Arc<SettingsManager>,
    pub kanban_manager: Arc<KanbanManager>,
    pub scanner: Arc<RwLock<Scanner>>,
}

impl AppState {
    pub fn new() -> anyhow::Result<Self> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not find config directory"))?
            .join("git-projects-manager");

        std::fs::create_dir_all(&config_dir)?;

        Ok(Self {
            config_manager: Arc::new(ConfigManager::new()?),
            settings_manager: Arc::new(SettingsManager::new()?),
            kanban_manager: Arc::new(KanbanManager::new(&config_dir)),
            scanner: Arc::new(RwLock::new(Scanner::new())),
        })
    }
}
