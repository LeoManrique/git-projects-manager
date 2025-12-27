use crate::domain::settings::SettingsManager;
use crate::domain::{AppSettings, EditorApp, OpenMethod, TerminalApp};
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

    match &terminal.open_method {
        OpenMethod::AppleScript { script } => {
            let escaped_path = path.replace("'", "'\\''");
            let final_script = script.replace("{path}", &escaped_path);
            Command::new("osascript")
                .args(["-e", &final_script])
                .spawn()
                .map_err(|e| e.to_string())?;
        }
        OpenMethod::Open | OpenMethod::Default => {
            Command::new("open")
                .args(["-a", &terminal.name, &path])
                .spawn()
                .map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}

#[tauri::command]
pub fn open_in_editor(path: String, editor_id: String) -> Result<(), String> {
    let manager = SettingsManager::new().map_err(|e| e.to_string())?;
    let editors = manager.get_available_editors();

    let editor = editors
        .iter()
        .find(|e| e.id == editor_id)
        .ok_or_else(|| format!("Editor '{}' not found", editor_id))?;

    Command::new("open")
        .args(["-a", &editor.name, &path])
        .spawn()
        .map_err(|e| e.to_string())?;

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
pub fn set_default_editor(editor_id: Option<String>) -> Result<(), String> {
    let manager = SettingsManager::new().map_err(|e| e.to_string())?;
    manager
        .set_default_editor(editor_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_available_terminals() -> Result<Vec<TerminalApp>, String> {
    let manager = SettingsManager::new().map_err(|e| e.to_string())?;
    Ok(manager.get_available_terminals())
}

#[tauri::command]
pub fn get_available_editors() -> Result<Vec<EditorApp>, String> {
    let manager = SettingsManager::new().map_err(|e| e.to_string())?;
    Ok(manager.get_available_editors())
}
