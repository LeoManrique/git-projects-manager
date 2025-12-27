use crate::domain::settings::SettingsManager;
use crate::domain::{AppSettings, TerminalApp};
use std::process::Command;

#[tauri::command]
pub fn get_app_settings() -> Result<AppSettings, String> {
    let manager = SettingsManager::new().map_err(|e| e.to_string())?;
    manager.load().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_in_terminal(path: String, terminal_id: String) -> Result<(), String> {
    let manager = SettingsManager::new().map_err(|e| e.to_string())?;
    let terminals = manager.get_available_terminals();

    let terminal = terminals
        .iter()
        .find(|t| t.id == terminal_id)
        .ok_or_else(|| format!("Terminal '{}' not found", terminal_id))?;

    // Use macOS open command to launch terminal at the specified directory
    match terminal_id.as_str() {
        "iterm2" => {
            // iTerm2 supports opening a directory directly
            Command::new("open")
                .args(["-a", &terminal.name, &path])
                .spawn()
                .map_err(|e| e.to_string())?;
        }
        "terminal" => {
            // Terminal.app needs AppleScript to open in a specific directory
            let script = format!(
                r#"tell application "Terminal"
                    activate
                    do script "cd '{}'"
                end tell"#,
                path.replace("'", "'\\''")
            );
            Command::new("osascript")
                .args(["-e", &script])
                .spawn()
                .map_err(|e| e.to_string())?;
        }
        _ => {
            // Fallback: try to open with the app directly
            Command::new("open")
                .args(["-a", &terminal.name, &path])
                .spawn()
                .map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}

#[tauri::command]
pub fn set_default_terminal(terminal_id: Option<String>) -> Result<(), String> {
    let manager = SettingsManager::new().map_err(|e| e.to_string())?;
    manager
        .set_default_terminal(terminal_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_available_terminals() -> Result<Vec<TerminalApp>, String> {
    let manager = SettingsManager::new().map_err(|e| e.to_string())?;
    Ok(manager.get_available_terminals())
}
