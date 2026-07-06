use gpm_core::domain::MonitoredFolder;
use gpm_core::AppState;
use tauri::State;

#[tauri::command]
pub async fn get_monitored_folders(
    state: State<'_, AppState>,
) -> Result<Vec<MonitoredFolder>, String> {
    state
        .config_manager
        .get_folders()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_monitored_folder(
    path: String,
    name: String,
    only_local_checks: bool,
    state: State<'_, AppState>,
) -> Result<MonitoredFolder, String> {
    state
        .config_manager
        .add_folder(path, name, only_local_checks)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_monitored_folder(
    id: String,
    path: String,
    name: String,
    only_local_checks: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state
        .config_manager
        .update_folder(id, path, name, only_local_checks)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_monitored_folder(
    id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state
        .config_manager
        .delete_folder(id)
        .map_err(|e| e.to_string())
}
