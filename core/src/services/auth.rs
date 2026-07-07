//! Google sign-in/out flows for kanban cloud sync.

use crate::AppState;
use crate::config;
use crate::domain::auth::SyncUser;
use crate::infrastructure::oauth;
use anyhow::Result;

/// Run the Google loopback-PKCE flow in the system browser (opened via
/// `open_url`), exchange the resulting ID token for a server session, and
/// persist it.
///
/// # Errors
/// Returns an error if the OAuth flow fails or times out, or if the sync
/// server rejects the sign-in.
pub async fn sign_in(
    state: &AppState,
    open_url: impl FnOnce(&str) -> Result<()>,
) -> Result<SyncUser> {
    let id_token = oauth::run_loopback_pkce(
        config::oauth_client_id(),
        config::oauth_client_secret(),
        open_url,
    )
    .await?;

    let session = state.sync_client.sign_in(&id_token).await?;

    if let Err(e) = state.token_store.save(&session) {
        tracing::warn!(?e, "failed to persist sync session to token store");
    }

    let user = session.user.clone();
    *state.auth.write() = Some(session);
    Ok(user)
}

/// Best-effort server sign-out, then clear the local session unconditionally.
pub async fn sign_out(state: &AppState) {
    let token = state.auth.read().as_ref().map(|s| s.token.clone());
    if let Some(t) = token
        && let Err(e) = state.sync_client.sign_out(&t).await
    {
        tracing::warn!(?e, "server sign-out call failed; clearing local session anyway");
    }
    if let Err(e) = state.token_store.clear() {
        tracing::warn!(?e, "failed to clear token store");
    }
    *state.auth.write() = None;
}

/// The currently signed-in sync user, if any (pure in-memory read).
#[must_use]
pub fn current_user(state: &AppState) -> Option<SyncUser> {
    state.auth.read().as_ref().map(|s| s.user.clone())
}
