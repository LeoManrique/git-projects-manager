use crate::config;
use crate::domain::auth::SyncUser;
use crate::infrastructure::oauth;
use crate::state::AppState;
use anyhow::{Result, anyhow};
use tauri::{AppHandle, State};
use tauri_plugin_shell::ShellExt;

#[tauri::command]
pub async fn sign_in_with_google(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<SyncUser, String> {
    let client_id = config::oauth_client_id();

    let opener = app.clone();
    let id_token = oauth::run_loopback_pkce(client_id, move |url| -> Result<()> {
        #[allow(deprecated)]
        opener
            .shell()
            .open(url, None)
            .map_err(|e| anyhow!("failed to open browser: {e}"))
    })
    .await
    .map_err(|e| e.to_string())?;

    let session = state
        .sync_client
        .sign_in(&id_token)
        .await
        .map_err(|e| e.to_string())?;

    if let Err(e) = state.token_store.save(&session) {
        tracing::warn!(?e, "failed to persist sync session to token store");
    }

    let user = session.user.clone();
    *state.auth.write() = Some(session);
    Ok(user)
}

#[tauri::command]
pub async fn sign_out(state: State<'_, AppState>) -> Result<(), String> {
    let token = state.auth.read().as_ref().map(|s| s.token.clone());
    if let Some(t) = token {
        if let Err(e) = state.sync_client.sign_out(&t).await {
            tracing::warn!(?e, "server sign-out call failed; clearing local session anyway");
        }
    }
    if let Err(e) = state.token_store.clear() {
        tracing::warn!(?e, "failed to clear token store");
    }
    *state.auth.write() = None;
    Ok(())
}

#[tauri::command]
pub fn get_sync_user(state: State<'_, AppState>) -> Option<SyncUser> {
    state.auth.read().as_ref().map(|s| s.user.clone())
}
