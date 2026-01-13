use crate::domain::kanban::KanbanState;
use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub fn get_kanban_state(state: State<AppState>) -> Result<KanbanState, String> {
    state.kanban_manager.load().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn move_kanban_card(
    state: State<AppState>,
    repo_path: String,
    to_column: String,
) -> Result<KanbanState, String> {
    state
        .kanban_manager
        .move_card(&repo_path, &to_column)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_kanban_notes(
    state: State<AppState>,
    repo_path: String,
    notes: Option<String>,
) -> Result<KanbanState, String> {
    state
        .kanban_manager
        .update_notes(&repo_path, notes)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remove_kanban_card(
    state: State<AppState>,
    repo_path: String,
) -> Result<KanbanState, String> {
    state
        .kanban_manager
        .remove_card(&repo_path)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn sync_kanban_with_repos(
    state: State<AppState>,
    repo_paths: Vec<String>,
) -> Result<KanbanState, String> {
    state
        .kanban_manager
        .sync_with_repos(repo_paths)
        .map_err(|e| e.to_string())
}
