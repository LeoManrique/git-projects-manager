use crate::domain::{Config, MonitoredFolder};
use crate::infrastructure::app_dir::app_data_dir;
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
        let config_path = app_data_dir()?.join("config.json");

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
    /// Returns an error if the path overlaps a folder that is already
    /// monitored, or if the config cannot be loaded or saved back to disk.
    pub fn add_folder(
        &self,
        path: String,
        name: String,
        only_local_checks: bool,
        detect_uninitialized: bool,
    ) -> Result<MonitoredFolder> {
        let mut config = self.load()?;
        Self::reject_overlap(&config, &path, None)?;
        let folder = MonitoredFolder::new(path, name, only_local_checks, detect_uninitialized);
        config.folders.push(folder.clone());
        self.save(&config)?;
        Ok(folder)
    }

    /// Refuse a path that would make two monitored folders scan the same
    /// repositories concurrently (see `FolderOverlap`).
    fn reject_overlap(config: &Config, path: &str, skip_id: Option<&str>) -> Result<()> {
        // Resolved to a message before returning so the borrow of `config` ends
        // here, leaving the caller free to mutate it.
        let conflict = config
            .overlapping_folder(path, skip_id)
            .map(|(existing, overlap)| overlap.message(existing));

        match conflict {
            Some(message) => Err(anyhow::anyhow!(message)),
            None => Ok(()),
        }
    }

    /// Returns the folder's previous path when the update changed it.
    ///
    /// # Errors
    /// Returns an error if no folder has the given `id`, if the new path
    /// overlaps another monitored folder, or if the config cannot be loaded or
    /// saved.
    // `id` stays owned: pub API consumed with owned Strings by the desktop crate.
    #[allow(clippy::needless_pass_by_value)]
    pub fn update_folder(
        &self,
        id: String,
        path: String,
        name: String,
        only_local_checks: bool,
        detect_uninitialized: bool,
    ) -> Result<Option<String>> {
        let mut config = self.load()?;
        Self::reject_overlap(&config, &path, Some(&id))?;

        let Some(folder) = config.folders.iter_mut().find(|f| f.id == id) else {
            return Err(anyhow::anyhow!("Folder not found"));
        };
        let replaced_path =
            (folder.path != path).then(|| std::mem::replace(&mut folder.path, path));
        folder.name = name;
        folder.only_local_checks = only_local_checks;
        folder.detect_uninitialized = detect_uninitialized;
        self.save(&config)?;

        Ok(replaced_path)
    }

    /// Returns the removed folder, or `None` when no folder has the given `id`.
    ///
    /// # Errors
    /// Returns an error if the config cannot be loaded or saved.
    // `id` stays owned: pub API consumed with owned Strings by the desktop crate.
    #[allow(clippy::needless_pass_by_value)]
    pub fn delete_folder(&self, id: String) -> Result<Option<MonitoredFolder>> {
        let mut config = self.load()?;
        let removed = config
            .folders
            .iter()
            .position(|f| f.id == id)
            .map(|i| config.folders.remove(i));
        self.save(&config)?;
        Ok(removed)
    }

    /// # Errors
    /// Returns an error if the config cannot be loaded.
    pub fn get_folders(&self) -> Result<Vec<MonitoredFolder>> {
        let config = self.load()?;
        Ok(config.folders)
    }
}
