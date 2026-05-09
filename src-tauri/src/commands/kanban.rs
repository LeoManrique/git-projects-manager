use crate::domain::kanban::KanbanState;
use crate::infrastructure::github_cli::{self, GhAuthStatus, GhRepo};
use crate::state::AppState;
use serde::Serialize;
use tauri::State;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KanbanRefresh {
    pub repos: Vec<GhRepo>,
    pub state: KanbanState,
}

#[tauri::command]
pub fn check_gh_auth() -> GhAuthStatus {
    github_cli::check_auth()
}

#[tauri::command]
pub fn refresh_kanban(state: State<AppState>) -> Result<KanbanRefresh, String> {
    let repos = github_cli::list_repos().map_err(|e| e.to_string())?;
    let names: Vec<String> = repos.iter().map(|r| r.name_with_owner.clone()).collect();
    let kanban_state = state
        .kanban_manager
        .sync_with_repos(names)
        .map_err(|e| e.to_string())?;
    Ok(KanbanRefresh { repos, state: kanban_state })
}

#[tauri::command]
pub fn move_kanban_card(
    state: State<AppState>,
    name_with_owner: String,
    to_column: String,
) -> Result<KanbanState, String> {
    state
        .kanban_manager
        .move_card(&name_with_owner, &to_column)
        .map_err(|e| e.to_string())
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
    Ok(KanbanRefresh { repos, state: kanban_state })
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
