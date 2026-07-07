use anyhow::{Result, anyhow};
use gpm_core::AppState;
use gpm_core::domain::auth::SyncUser;
use gpm_core::services::auth;
use tauri::{AppHandle, State};
use tauri_plugin_shell::ShellExt;

#[tauri::command]
pub async fn sign_in_with_google(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<SyncUser, String> {
    auth::sign_in(state.inner(), move |url| -> Result<()> {
        #[allow(deprecated)]
        app.shell()
            .open(url, None)
            .map_err(|e| anyhow!("failed to open browser: {e}"))
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn sign_out(state: State<'_, AppState>) -> Result<(), String> {
    auth::sign_out(state.inner()).await;
    Ok(())
}

#[tauri::command]
// Tauri's invoke layer hands commands owned State/args; the signature is the command contract.
#[allow(clippy::needless_pass_by_value)]
pub fn get_sync_user(state: State<'_, AppState>) -> Option<SyncUser> {
    auth::current_user(state.inner())
}
