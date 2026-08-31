use gpm_core::domain::{GitCleanResult, ScanResult};
use gpm_core::infrastructure::git::GitOperations;
use gpm_core::AppState;
use std::path::PathBuf;
use tauri::State;

/// Run a blocking body on the runtime's blocking pool.
///
/// Every command in this module is `async` but does nothing but block: git
/// subprocesses, network fetches and filesystem walks. Left on a tokio worker,
/// one Clean All saturates the runtime and every unrelated command queues
/// behind it. The macOS FFI bridge has always done this; the Tauri side did not.
async fn blocking<T, F>(f: F) -> Result<T, String>
where
    F: FnOnce() -> Result<T, String> + Send + 'static,
    T: Send + 'static,
{
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| format!("task failed: {e}"))?
}

#[tauri::command]
pub async fn pull_repo(path: String) -> Result<String, String> {
    blocking(move || GitOperations::pull(&PathBuf::from(path)).map_err(|e| e.to_string())).await
}

#[tauri::command]
pub async fn clean_repo(path: String, state: State<'_, AppState>) -> Result<GitCleanResult, String> {
    let settings_manager = state.settings_manager.clone();
    blocking(move || {
        let settings = settings_manager.get_git_clean_settings();
        let (files_removed, directories_removed) =
            GitOperations::clean(&PathBuf::from(path), &settings.exclude_patterns)
                .map_err(|e| e.to_string())?;

        Ok(GitCleanResult {
            files_removed,
            directories_removed,
        })
    })
    .await
}

#[tauri::command]
pub async fn scan_folder(
    path: String,
    only_local_checks: bool,
    state: State<'_, AppState>,
) -> Result<ScanResult, String> {
    let scanner = state.scanner.clone();
    blocking(move || {
        let scanner = scanner.read();
        Ok(scanner.scan_folder(&PathBuf::from(path), only_local_checks))
    })
    .await
}

#[tauri::command]
pub async fn cancel_scan(state: State<'_, AppState>) -> Result<(), String> {
    let scanner_lock = state.scanner.clone();
    blocking(move || {
        scanner_lock.read().cancel();

        // Create new scanner for next scan. This blocks until every in-flight
        // scan has released its read guard, which is why it must not run on a
        // runtime worker.
        *scanner_lock.write() = gpm_core::domain::scanner::Scanner::new();

        Ok(())
    })
    .await
}
