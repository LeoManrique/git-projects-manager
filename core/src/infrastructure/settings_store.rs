use crate::domain::{AppSettings, EditorApp, GitCleanSettings, TerminalApp};
use anyhow::Result;
use std::fs;
use std::path::PathBuf;

const TERMINALS_JSON: &str = include_str!("../../resources/terminals.json");
const EDITORS_JSON: &str = include_str!("../../resources/editors.json");

pub struct SettingsManager {
    settings_path: PathBuf,
}

impl SettingsManager {
    /// # Errors
    /// Returns an error if the platform config directory cannot be
    /// determined or the app config directory cannot be created.
    pub fn new() -> Result<Self> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not find config directory"))?
            .join("git-projects-manager");

        fs::create_dir_all(&config_dir)?;

        let settings_path = config_dir.join("settings.json");

        Ok(Self { settings_path })
    }

    /// # Errors
    /// Returns an error if `settings.json` cannot be read or parsed.
    pub fn load(&self) -> Result<AppSettings> {
        if !self.settings_path.exists() {
            return Ok(AppSettings::default());
        }

        let content = fs::read_to_string(&self.settings_path)?;
        let settings: AppSettings = serde_json::from_str(&content)?;
        Ok(settings)
    }

    /// # Errors
    /// Returns an error if serialization or the atomic write to disk fails.
    pub fn save(&self, settings: &AppSettings) -> Result<()> {
        let content = serde_json::to_string_pretty(settings)?;
        crate::infrastructure::atomic_write::write_atomic(&self.settings_path, &content)?;
        Ok(())
    }

    /// # Errors
    /// Returns an error if the settings cannot be loaded or saved.
    pub fn set_default_terminal(&self, terminal_id: Option<String>) -> Result<()> {
        let mut settings = self.load()?;
        settings.default_terminal = terminal_id;
        self.save(&settings)?;
        Ok(())
    }

    /// # Errors
    /// Returns an error if the settings cannot be loaded or saved.
    pub fn set_default_editor(&self, editor_id: Option<String>) -> Result<()> {
        let mut settings = self.load()?;
        settings.default_editor = editor_id;
        self.save(&settings)?;
        Ok(())
    }

    /// # Panics
    /// Panics if the bundled `terminals.json` resource is not valid JSON
    /// (a build-time defect, not a runtime condition).
    #[must_use]
    pub fn get_available_terminals(&self) -> Vec<TerminalApp> {
        let terminals: Vec<TerminalApp> =
            serde_json::from_str(TERMINALS_JSON).expect("Invalid terminals.json");
        terminals
            .into_iter()
            .filter(|t| std::path::Path::new(&t.path).exists())
            .collect()
    }

    /// # Panics
    /// Panics if the bundled `editors.json` resource is not valid JSON
    /// (a build-time defect, not a runtime condition).
    #[must_use]
    pub fn get_available_editors(&self) -> Vec<EditorApp> {
        let editors: Vec<EditorApp> =
            serde_json::from_str(EDITORS_JSON).expect("Invalid editors.json");
        editors
            .into_iter()
            .filter(|e| std::path::Path::new(&e.path).exists())
            .collect()
    }

    #[must_use]
    pub fn get_git_clean_settings(&self) -> GitCleanSettings {
        self.load()
            .ok()
            .and_then(|s| s.git_clean_settings)
            .unwrap_or_default()
    }

    /// # Errors
    /// Returns an error if the settings cannot be loaded or saved.
    pub fn set_git_clean_settings(&self, settings: GitCleanSettings) -> Result<()> {
        let mut app_settings = self.load()?;
        app_settings.git_clean_settings = Some(settings);
        self.save(&app_settings)?;
        Ok(())
    }
}
