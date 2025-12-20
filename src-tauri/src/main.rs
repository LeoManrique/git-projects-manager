#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod domain;
mod infrastructure;
mod state;

use state::AppState;
use tauri::Manager;

fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let state = AppState::new()
                .expect("Failed to initialize application state");

            app.manage(state);

            #[cfg(debug_assertions)]
            {
                let window = app.get_webview_window("main").unwrap();
                window.open_devtools();
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::folder::get_monitored_folders,
            commands::folder::add_monitored_folder,
            commands::folder::update_monitored_folder,
            commands::folder::delete_monitored_folder,
            commands::scan::scan_folder,
            commands::scan::cancel_scan,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
