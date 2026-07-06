use gpm_core::domain::{AppSettings, EditorApp, GitCleanSettings, TerminalApp};
use gpm_core::infrastructure::launcher;
use gpm_core::AppState;
use tauri::State;

#[tauri::command]
// Tauri's invoke layer hands commands owned State/args; the signature is the command contract.
#[allow(clippy::needless_pass_by_value)]
pub fn get_app_settings(state: State<AppState>) -> Result<AppSettings, String> {
    state.settings_manager.load().map_err(|e| e.to_string())
}

#[tauri::command]
// Tauri's invoke layer hands commands owned State/args; the signature is the command contract.
#[allow(clippy::needless_pass_by_value)]
pub fn open_in_terminal(path: String, terminal_id: String, state: State<AppState>) -> Result<(), String> {
    launcher::open_terminal_by_id(&state.settings_manager, &terminal_id, &path).map_err(|e| e.to_string())
}

#[tauri::command]
// Tauri's invoke layer hands commands owned State/args; the signature is the command contract.
#[allow(clippy::needless_pass_by_value)]
pub fn open_in_lms_github(path: String) -> Result<(), String> {
    launcher::open_in_lms_github(&path).map_err(|e| e.to_string())
}

#[tauri::command]
// Tauri's invoke layer hands commands owned State/args; the signature is the command contract.
#[allow(clippy::needless_pass_by_value)]
pub fn open_in_editor(path: String, editor_id: String, state: State<AppState>) -> Result<(), String> {
    launcher::open_editor_by_id(&state.settings_manager, &editor_id, &path).map_err(|e| e.to_string())
}

#[tauri::command]
// Tauri's invoke layer hands commands owned State/args; the signature is the command contract.
#[allow(clippy::needless_pass_by_value)]
pub fn set_default_terminal(terminal_id: Option<String>, state: State<AppState>) -> Result<(), String> {
    state
        .settings_manager
        .set_default_terminal(terminal_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
// Tauri's invoke layer hands commands owned State/args; the signature is the command contract.
#[allow(clippy::needless_pass_by_value)]
pub fn set_default_editor(editor_id: Option<String>, state: State<AppState>) -> Result<(), String> {
    state
        .settings_manager
        .set_default_editor(editor_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
// Owned State is Tauri's contract; Result<_, String> is what the frontend invoke call expects.
#[allow(clippy::needless_pass_by_value, clippy::unnecessary_wraps)]
pub fn get_available_terminals(state: State<AppState>) -> Result<Vec<TerminalApp>, String> {
    Ok(state.settings_manager.get_available_terminals())
}

#[tauri::command]
// Owned State is Tauri's contract; Result<_, String> is what the frontend invoke call expects.
#[allow(clippy::needless_pass_by_value, clippy::unnecessary_wraps)]
pub fn get_available_editors(state: State<AppState>) -> Result<Vec<EditorApp>, String> {
    Ok(state.settings_manager.get_available_editors())
}

#[tauri::command]
// Owned State is Tauri's contract; Result<_, String> is what the frontend invoke call expects.
#[allow(clippy::needless_pass_by_value, clippy::unnecessary_wraps)]
pub fn get_git_clean_settings(state: State<AppState>) -> Result<GitCleanSettings, String> {
    Ok(state.settings_manager.get_git_clean_settings())
}

#[tauri::command]
// Tauri's invoke layer hands commands owned State/args; the signature is the command contract.
#[allow(clippy::needless_pass_by_value)]
pub fn set_git_clean_settings(
    settings: GitCleanSettings,
    state: State<AppState>,
) -> Result<(), String> {
    state
        .settings_manager
        .set_git_clean_settings(settings)
        .map_err(|e| e.to_string())
}
