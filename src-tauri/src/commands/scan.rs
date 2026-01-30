use crate::domain::{GitCleanResult, ScanResult};
use crate::infrastructure::git::GitOperations;
use crate::state::AppState;
use std::path::Path;
use tauri::State;

#[tauri::command]
pub async fn pull_repo(path: String) -> Result<String, String> {
    GitOperations::pull(Path::new(&path)).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn clean_repo(path: String, state: State<'_, AppState>) -> Result<GitCleanResult, String> {
    let settings = state.settings_manager.get_git_clean_settings();
    let (files_removed, directories_removed) =
        GitOperations::clean(Path::new(&path), &settings.exclude_patterns)
            .map_err(|e| e.to_string())?;

    Ok(GitCleanResult {
        files_removed,
        directories_removed,
    })
}

#[tauri::command]
pub async fn scan_folder(
    path: String,
    only_local_checks: bool,
    state: State<'_, AppState>,
) -> Result<ScanResult, String> {
    let scanner = state.scanner.read();
    let result = scanner.scan_folder(Path::new(&path), only_local_checks);
    Ok(result)
}

#[tauri::command]
pub async fn cancel_scan(state: State<'_, AppState>) -> Result<(), String> {
    let scanner = state.scanner.read();
    scanner.cancel();

    // Create new scanner for next scan
    drop(scanner);
    let mut scanner = state.scanner.write();
    *scanner = crate::domain::scanner::Scanner::new();

    Ok(())
}
