use crate::domain::{config::ConfigManager, scanner::Scanner, settings::SettingsManager};
use parking_lot::RwLock;
use std::sync::Arc;

pub struct AppState {
    pub config_manager: Arc<ConfigManager>,
    pub settings_manager: Arc<SettingsManager>,
    pub scanner: Arc<RwLock<Scanner>>,
}

impl AppState {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            config_manager: Arc::new(ConfigManager::new()?),
            settings_manager: Arc::new(SettingsManager::new()?),
            scanner: Arc::new(RwLock::new(Scanner::new())),
        })
    }
}
