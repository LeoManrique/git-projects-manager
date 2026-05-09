use crate::domain::auth::SyncStatus;
use crate::domain::kanban::{KanbanCard, KanbanState};
use crate::infrastructure::github_cli::{self, GhAuthStatus, GhRepo};
use crate::infrastructure::sync_client::SyncOutcome;
use crate::state::AppState;
use serde::Serialize;
use std::collections::HashSet;
use std::sync::Arc;
use tauri::{Manager, State};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KanbanRefresh {
    pub repos: Vec<GhRepo>,
    pub state: KanbanState,
    pub sync_status: SyncStatus,
}

#[tauri::command]
pub fn check_gh_auth() -> GhAuthStatus {
    github_cli::check_auth()
}

#[tauri::command]
pub async fn refresh_kanban(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<KanbanRefresh, String> {
    let repos = github_cli::list_repos().map_err(|e| e.to_string())?;
    let names: Vec<String> = repos.iter().map(|r| r.name_with_owner.clone()).collect();
    let local_state = state
        .kanban_manager
        .sync_with_repos(names.clone())
        .map_err(|e| e.to_string())?;

    let token = state.auth.read().as_ref().map(|s| s.token.clone());
    let (final_state, sync_status) = match token {
        None => (local_state, SyncStatus::Disabled),
        Some(token) => {
            let cards: Vec<KanbanCard> = local_state.cards.values().cloned().collect();
            match state.sync_client.sync(&token, &cards).await {
                SyncOutcome::Ok(merged) => {
                    let names_set: HashSet<String> = names.iter().cloned().collect();
                    let merged = merge_with_local(local_state, merged, &names_set);
                    state.kanban_manager.save(&merged).map_err(|e| e.to_string())?;
                    (merged, SyncStatus::Synced)
                }
                SyncOutcome::Unauthorized => {
                    let app_state = app.state::<AppState>();
                    if let Err(e) = app_state.token_store.clear() {
                        tracing::warn!(?e, "failed to clear expired token from store");
                    }
                    *app_state.auth.write() = None;
                    (local_state, SyncStatus::Expired)
                }
                SyncOutcome::Network(e) => {
                    tracing::warn!(error = %e, "kanban sync failed; staying offline");
                    (local_state, SyncStatus::Offline)
                }
            }
        }
    };

    Ok(KanbanRefresh {
        repos,
        state: final_state,
        sync_status,
    })
}

#[tauri::command]
pub fn move_kanban_card(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    name_with_owner: String,
    to_column: String,
) -> Result<KanbanState, String> {
    let new_state = state
        .kanban_manager
        .move_card(&name_with_owner, &to_column)
        .map_err(|e| e.to_string())?;

    if let Some(card) = new_state.cards.get(&name_with_owner).cloned() {
        let token = state.auth.read().as_ref().map(|s| s.token.clone());
        if let Some(token) = token {
            let app_handle = app.clone();
            let sync_client = Arc::clone(&state.sync_client);
            tauri::async_runtime::spawn(async move {
                let outcome = sync_client.sync(&token, &[card]).await;
                if matches!(outcome, SyncOutcome::Unauthorized) {
                    let s = app_handle.state::<AppState>();
                    let _ = s.token_store.clear();
                    *s.auth.write() = None;
                }
            });
        }
    }

    Ok(new_state)
}

#[tauri::command]
pub fn delete_github_repo(
    state: State<AppState>,
    name_with_owner: String,
) -> Result<KanbanRefresh, String> {
    github_cli::delete_repo(&name_with_owner).map_err(|e| e.to_string())?;
    let repos = github_cli::list_repos().map_err(|e| e.to_string())?;
    let names: Vec<String> = repos.iter().map(|r| r.name_with_owner.clone()).collect();
    let kanban_state = state
        .kanban_manager
        .sync_with_repos(names)
        .map_err(|e| e.to_string())?;
    let sync_status = if state.auth.read().is_some() {
        SyncStatus::Synced
    } else {
        SyncStatus::Disabled
    };
    Ok(KanbanRefresh {
        repos,
        state: kanban_state,
        sync_status,
    })
}

#[tauri::command]
pub fn open_url(url: String) -> Result<(), String> {
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err("only http(s) URLs are allowed".into());
    }
    std::process::Command::new("open")
        .arg(&url)
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Build the final kanban state by adopting server's authoritative card data
/// for repos in `gh_names`, preserving local entries for repos the server
/// hasn't seen yet.
fn merge_with_local(
    mut local: KanbanState,
    remote: Vec<KanbanCard>,
    gh_names: &HashSet<String>,
) -> KanbanState {
    for card in remote {
        if gh_names.contains(&card.name_with_owner) {
            local.cards.insert(card.name_with_owner.clone(), card);
        }
    }
    local
}
