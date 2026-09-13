//! Kanban orchestration: GitHub repo listing, board reconciliation, cloud
//! sync, and the offline cache.

use crate::AppState;
use crate::domain::auth::{SyncSession, SyncStatus};
use crate::domain::kanban::{KanbanCard, KanbanState};
use crate::infrastructure::github_cli::{self, GhRepo};
use crate::infrastructure::kanban_store::KanbanManager;
use crate::infrastructure::repos_cache::ReposCache;
use crate::infrastructure::sync_client::SyncOutcome;
use crate::infrastructure::token_store::TokenStore;
use anyhow::Result;
use parking_lot::RwLock;
use serde::Serialize;
use std::collections::HashSet;
use std::sync::Arc;

/// Everything a frontend needs to render the kanban board.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KanbanRefresh {
    pub repos: Vec<GhRepo>,
    pub state: KanbanState,
    pub sync_status: SyncStatus,
}

/// Full refresh: list GitHub repos, reconcile the local board with them,
/// push/pull cloud state when signed in, and persist the offline cache.
/// Sync failures degrade to an offline/expired status instead of erroring.
///
/// # Errors
/// Returns an error if the `gh` repo listing fails or the kanban store
/// cannot be read or written.
pub async fn refresh(state: &AppState) -> Result<KanbanRefresh> {
    let repos = tokio::task::spawn_blocking(github_cli::list_repos).await??;
    let names: Vec<String> = repos.iter().map(|r| r.name_with_owner.clone()).collect();
    let local_state = state.kanban_manager.sync_with_repos(names.clone())?;

    let token = state.auth.read().as_ref().map(|s| s.token.clone());
    let (final_state, sync_status) = match token {
        None => (local_state, SyncStatus::Disabled),
        Some(token) => {
            let cards: Vec<KanbanCard> = local_state.cards.values().cloned().collect();
            match state.sync_client.sync(&token, &cards).await {
                SyncOutcome::Ok(remote) => {
                    let names_set: HashSet<String> = names.into_iter().collect();
                    // Re-merge against the CURRENT store under its lock — a
                    // card moved while this sync request was in flight must
                    // not be clobbered by the stale response.
                    let merged = state
                        .kanban_manager
                        .update(|current| merge_remote(current, remote, &names_set))?;
                    (merged, SyncStatus::Synced)
                }
                SyncOutcome::Unauthorized => {
                    clear_expired_session(state.token_store.as_ref(), &state.auth);
                    (local_state, SyncStatus::Expired)
                }
                SyncOutcome::Network(e) => {
                    tracing::warn!(error = %e, "kanban sync failed; staying offline");
                    (local_state, SyncStatus::Offline)
                }
            }
        }
    };

    persist_cache(state, repos.clone(), sync_status);

    Ok(KanbanRefresh {
        repos,
        state: final_state,
        sync_status,
    })
}

/// Offline-first load of the snapshot persisted by the last refresh.
/// Returns `None` when no refresh has ever completed on this machine.
///
/// # Errors
/// Returns an error if the cache or kanban store cannot be read or parsed.
pub fn load_local(state: &AppState) -> Result<Option<KanbanRefresh>> {
    let Some(cache) = state.repos_cache.load()? else {
        return Ok(None);
    };
    let kanban_state = state.kanban_manager.load()?;
    Ok(Some(KanbanRefresh {
        repos: cache.repos,
        state: kanban_state,
        sync_status: cache.sync_status,
    }))
}

/// Move a card to another column locally, then fire-and-forget a one-card
/// cloud sync (an expired session is cleared in the background).
///
/// # Errors
/// Returns an error if the kanban store cannot be read or written.
pub async fn move_card(
    state: &AppState,
    name_with_owner: &str,
    to_column: &str,
) -> Result<KanbanState> {
    let nwo = name_with_owner.to_string();
    let column = to_column.to_string();
    mutate_card(state, name_with_owner, move |manager| {
        manager.move_card(&nwo, &column)
    })
    .await
}

/// Replace a card's notes locally (trimmed; blank clears them), then
/// fire-and-forget a one-card cloud sync exactly like [`move_card`].
///
/// # Errors
/// Returns an error if the kanban store cannot be read or written.
pub async fn set_notes(
    state: &AppState,
    name_with_owner: &str,
    notes: Option<String>,
) -> Result<KanbanState> {
    let nwo = name_with_owner.to_string();
    mutate_card(state, name_with_owner, move |manager| {
        manager.update_notes(&nwo, notes)
    })
    .await
}

/// Delete a repository on GitHub (`gh repo delete`), then run a full refresh
/// so the board, cloud state, and cache all reflect the deletion — and the
/// reported sync status is a real outcome, not an assertion.
///
/// # Errors
/// Returns an error if the deletion or the subsequent refresh fails.
pub async fn delete_repo(state: &AppState, name_with_owner: &str) -> Result<KanbanRefresh> {
    let nwo = name_with_owner.to_string();
    tokio::task::spawn_blocking(move || github_cli::delete_repo(&nwo)).await??;
    refresh(state).await
}

/// Run one store mutation off the async runtime, then push the touched card
/// to the cloud in the background. The caller gets the new local state at
/// once, so the UI never waits on the network.
async fn mutate_card(
    state: &AppState,
    name_with_owner: &str,
    mutate: impl FnOnce(&KanbanManager) -> Result<KanbanState> + Send + 'static,
) -> Result<KanbanState> {
    let manager = Arc::clone(&state.kanban_manager);
    let new_state = tokio::task::spawn_blocking(move || mutate(&manager)).await??;
    if let Some(card) = new_state.cards.get(name_with_owner).cloned() {
        sync_card_in_background(state, card);
    }
    Ok(new_state)
}

/// Fire-and-forget one-card sync. Only an expired session gets a reaction
/// (it is cleared so the UI can offer to sign in again); a network failure
/// is left for the next full refresh to reconcile.
fn sync_card_in_background(state: &AppState, card: KanbanCard) {
    let token = state.auth.read().as_ref().map(|s| s.token.clone());
    let Some(token) = token else {
        return;
    };
    let sync_client = Arc::clone(&state.sync_client);
    let token_store = Arc::clone(&state.token_store);
    let auth = Arc::clone(&state.auth);
    tokio::spawn(async move {
        if matches!(
            sync_client.sync(&token, &[card]).await,
            SyncOutcome::Unauthorized
        ) {
            clear_expired_session(token_store.as_ref(), &auth);
        }
    });
}

/// Adopt the server's authoritative card data for repos in `gh_names` —
/// unless the local card is strictly newer (a move or a notes edit that
/// landed while the sync request was in flight). Local-only cards are
/// preserved.
fn merge_remote(current: &mut KanbanState, remote: Vec<KanbanCard>, gh_names: &HashSet<String>) {
    for card in remote {
        if !gh_names.contains(&card.name_with_owner) {
            continue;
        }
        let local_is_newer = current
            .cards
            .get(&card.name_with_owner)
            .is_some_and(|local| local.updated_at > card.updated_at);
        if !local_is_newer {
            current.cards.insert(card.name_with_owner.clone(), card);
        }
    }
}

fn persist_cache(state: &AppState, repos: Vec<GhRepo>, sync_status: SyncStatus) {
    if let Err(e) = state.repos_cache.save(&ReposCache {
        repos,
        sync_status,
        fetched_at: chrono::Utc::now().timestamp_millis(),
    }) {
        tracing::warn!(?e, "failed to persist repos cache");
    }
}

/// Forget a session the server no longer accepts. Takes the two pieces
/// instead of `AppState` so a detached background task can call it too.
fn clear_expired_session(token_store: &dyn TokenStore, auth: &RwLock<Option<SyncSession>>) {
    if let Err(e) = token_store.clear() {
        tracing::warn!(?e, "failed to clear expired token from store");
    }
    *auth.write() = None;
}

#[cfg(test)]
mod tests {
    use super::merge_remote;
    use crate::domain::kanban::{KanbanCard, KanbanState};
    use std::collections::HashSet;

    const REPO: &str = "leo/some-repo";

    fn card(notes: Option<&str>, updated_at: i64) -> KanbanCard {
        KanbanCard {
            name_with_owner: REPO.to_string(),
            column: "backlog".to_string(),
            notes: notes.map(str::to_string),
            created_at: 0,
            updated_at,
        }
    }

    fn board_with(local: KanbanCard) -> KanbanState {
        let mut state = KanbanState::default();
        state.cards.insert(local.name_with_owner.clone(), local);
        state
    }

    fn known_repos() -> HashSet<String> {
        HashSet::from([REPO.to_string()])
    }

    #[test]
    fn remote_notes_are_adopted_when_newer() {
        let mut current = board_with(card(None, 10));

        merge_remote(
            &mut current,
            vec![card(Some("from the other machine"), 20)],
            &known_repos(),
        );

        assert_eq!(
            current.cards[REPO].notes.as_deref(),
            Some("from the other machine")
        );
    }

    #[test]
    fn local_notes_win_when_strictly_newer() {
        let mut current = board_with(card(Some("typed here"), 30));

        merge_remote(&mut current, vec![card(None, 20)], &known_repos());

        assert_eq!(current.cards[REPO].notes.as_deref(), Some("typed here"));
    }

    #[test]
    fn a_newer_remote_clearing_notes_wins_too() {
        // Accepted trade-off of whole-card last-writer-wins: the newer card
        // is taken as a whole, even when that removes the notes.
        let mut current = board_with(card(Some("about to go"), 10));

        merge_remote(&mut current, vec![card(None, 20)], &known_repos());

        assert_eq!(current.cards[REPO].notes, None);
    }
}
