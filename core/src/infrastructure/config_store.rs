use crate::domain::{Config, MonitoredFolder};
use anyhow::Result;
use std::fs;
use std::path::PathBuf;

pub struct ConfigManager {
    config_path: PathBuf,
}

impl ConfigManager {
    /// # Errors
    /// Returns an error if the platform config directory cannot be
    /// determined or the app config directory cannot be created.
    pub fn new() -> Result<Self> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not find config directory"))?
            .join("git-projects-manager");

        fs::create_dir_all(&config_dir)?;

        let config_path = config_dir.join("config.json");

        Ok(Self { config_path })
    }

    /// # Errors
    /// Returns an error if `config.json` cannot be read or parsed.
    pub fn load(&self) -> Result<Config> {
        if !self.config_path.exists() {
            return Ok(Config { folders: vec![] });
        }

        let content = fs::read_to_string(&self.config_path)?;
        let config: Config = serde_json::from_str(&content)?;
        Ok(config)
    }

    /// # Errors
    /// Returns an error if serialization or the atomic write to disk fails.
    pub fn save(&self, config: &Config) -> Result<()> {
        let content = serde_json::to_string_pretty(config)?;
        crate::infrastructure::atomic_write::write_atomic(&self.config_path, &content)?;
        Ok(())
    }

    /// # Errors
    /// Returns an error if the config cannot be loaded or saved back to disk.
    pub fn add_folder(&self, path: String, name: String, only_local_checks: bool) -> Result<MonitoredFolder> {
        let mut config = self.load()?;
        let folder = MonitoredFolder::new(path, name, only_local_checks);
        config.folders.push(folder.clone());
        self.save(&config)?;
        Ok(folder)
    }

    /// # Errors
    /// Returns an error if no folder has the given `id`, or if the config
    /// cannot be loaded or saved.
    // `id` stays owned: pub API consumed with owned Strings by the desktop crate.
    #[allow(clippy::needless_pass_by_value)]
    pub fn update_folder(&self, id: String, path: String, name: String, only_local_checks: bool) -> Result<()> {
        let mut config = self.load()?;

        if let Some(folder) = config.folders.iter_mut().find(|f| f.id == id) {
            folder.path = path;
            folder.name = name;
            folder.only_local_checks = only_local_checks;
            self.save(&config)?;
        } else {
            return Err(anyhow::anyhow!("Folder not found"));
        }

        Ok(())
    }

    /// # Errors
    /// Returns an error if the config cannot be loaded or saved.
    // `id` stays owned: pub API consumed with owned Strings by the desktop crate.
    #[allow(clippy::needless_pass_by_value)]
    pub fn delete_folder(&self, id: String) -> Result<()> {
        let mut config = self.load()?;
        config.folders.retain(|f| f.id != id);
        self.save(&config)?;
        Ok(())
    }

    /// # Errors
    /// Returns an error if the config cannot be loaded.
    pub fn get_folders(&self) -> Result<Vec<MonitoredFolder>> {
        let config = self.load()?;
        Ok(config.folders)
    }
}
