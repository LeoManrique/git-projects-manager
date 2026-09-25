use gpm_core::infrastructure::logging::{self, LogLevel};

/// Record a line from the frontend: an error it showed or swallowed.
#[tauri::command]
// Tauri's invoke layer hands commands owned args; the signature is the command contract.
#[allow(clippy::needless_pass_by_value)]
pub fn log_message(level: LogLevel, message: String) {
    logging::frontend(level, &message);
}

/// Where the log files live.
#[tauri::command]
pub fn get_logs_folder() -> Result<String, String> {
    logging::logs_dir()
        .map(|dir| dir.display().to_string())
        .map_err(|e| e.to_string())
}

/// Open the logs folder in the system file manager.
#[tauri::command]
pub fn open_logs_folder() -> Result<(), String> {
    logging::open_logs_folder().map_err(|e| e.to_string())
}
