use crate::domain::{config::ConfigManager, scanner::Scanner};
use parking_lot::RwLock;
use std::sync::Arc;

pub struct AppState {
    pub config_manager: Arc<ConfigManager>,
    pub scanner: Arc<RwLock<Scanner>>,
}

impl AppState {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            config_manager: Arc::new(ConfigManager::new()?),
            scanner: Arc::new(RwLock::new(Scanner::new())),
        })
    }
}
