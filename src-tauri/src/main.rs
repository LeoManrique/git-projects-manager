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
            commands::scan::pull_repo,
            commands::scan::clean_repo,
            commands::settings::get_app_settings,
            commands::settings::set_default_terminal,
            commands::settings::set_default_editor,
            commands::settings::get_available_terminals,
            commands::settings::get_available_editors,
            commands::settings::open_in_terminal,
            commands::settings::open_in_editor,
            commands::settings::open_in_lms_github,
            commands::settings::get_git_clean_settings,
            commands::settings::set_git_clean_settings,
            commands::kanban::get_kanban_state,
            commands::kanban::move_kanban_card,
            commands::kanban::update_kanban_notes,
            commands::kanban::remove_kanban_card,
            commands::kanban::sync_kanban_with_repos,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
