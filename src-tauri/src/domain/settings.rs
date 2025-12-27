use super::{AppSettings, TerminalApp};
use anyhow::Result;
use std::fs;
use std::path::PathBuf;

pub struct SettingsManager {
    settings_path: PathBuf,
}

impl SettingsManager {
    pub fn new() -> Result<Self> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not find config directory"))?
            .join(".git-projects-manager");

        fs::create_dir_all(&config_dir)?;

        let settings_path = config_dir.join("settings.json");

        Ok(Self { settings_path })
    }

    pub fn load(&self) -> Result<AppSettings> {
        if !self.settings_path.exists() {
            return Ok(AppSettings::default());
        }

        let content = fs::read_to_string(&self.settings_path)?;
        let settings: AppSettings = serde_json::from_str(&content)?;
        Ok(settings)
    }

    pub fn save(&self, settings: &AppSettings) -> Result<()> {
        let content = serde_json::to_string_pretty(settings)?;
        fs::write(&self.settings_path, content)?;
        Ok(())
    }

    pub fn set_default_terminal(&self, terminal_id: Option<String>) -> Result<()> {
        let mut settings = self.load()?;
        settings.default_terminal = terminal_id;
        self.save(&settings)?;
        Ok(())
    }

    pub fn get_available_terminals(&self) -> Vec<TerminalApp> {
        let mut terminals = Vec::new();

        // Check for iTerm2
        let iterm_path = "/Applications/iTerm.app";
        if std::path::Path::new(iterm_path).exists() {
            terminals.push(TerminalApp {
                id: "iterm2".to_string(),
                name: "iTerm".to_string(),
                path: iterm_path.to_string(),
            });
        }

        // Check for Terminal.app (macOS built-in)
        let terminal_path = "/System/Applications/Utilities/Terminal.app";
        if std::path::Path::new(terminal_path).exists() {
            terminals.push(TerminalApp {
                id: "terminal".to_string(),
                name: "Terminal".to_string(),
                path: terminal_path.to_string(),
            });
        }

        // Fallback for older macOS versions
        let terminal_path_legacy = "/Applications/Utilities/Terminal.app";
        if terminals.iter().all(|t| t.id != "terminal")
            && std::path::Path::new(terminal_path_legacy).exists()
        {
            terminals.push(TerminalApp {
                id: "terminal".to_string(),
                name: "Terminal".to_string(),
                path: terminal_path_legacy.to_string(),
            });
        }

        terminals
    }
}
