use crate::domain::{AppSettings, EditorApp, GitCleanSettings, OpenMethod, TerminalApp};
use crate::state::AppState;
use std::process::Command;
use tauri::State;

#[tauri::command]
pub fn get_app_settings(state: State<AppState>) -> Result<AppSettings, String> {
    state.settings_manager.load().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_in_terminal(path: String, terminal_id: String, state: State<AppState>) -> Result<(), String> {
    let terminals = state.settings_manager.get_available_terminals();

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
pub fn open_in_lms_github(path: String) -> Result<(), String> {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
    Command::new(shell)
        .args(["-lc", "exec lms-github \"$0\"", &path])
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn open_in_editor(path: String, editor_id: String, state: State<AppState>) -> Result<(), String> {
    let editors = state.settings_manager.get_available_editors();

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
pub fn set_default_terminal(terminal_id: Option<String>, state: State<AppState>) -> Result<(), String> {
    state
        .settings_manager
        .set_default_terminal(terminal_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_default_editor(editor_id: Option<String>, state: State<AppState>) -> Result<(), String> {
    state
        .settings_manager
        .set_default_editor(editor_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_available_terminals(state: State<AppState>) -> Result<Vec<TerminalApp>, String> {
    Ok(state.settings_manager.get_available_terminals())
}

#[tauri::command]
pub fn get_available_editors(state: State<AppState>) -> Result<Vec<EditorApp>, String> {
    Ok(state.settings_manager.get_available_editors())
}

#[tauri::command]
pub fn get_git_clean_settings(state: State<AppState>) -> Result<GitCleanSettings, String> {
    Ok(state.settings_manager.get_git_clean_settings())
}

#[tauri::command]
pub fn set_git_clean_settings(
    settings: GitCleanSettings,
    state: State<AppState>,
) -> Result<(), String> {
    state
        .settings_manager
        .set_git_clean_settings(settings)
        .map_err(|e| e.to_string())
}
