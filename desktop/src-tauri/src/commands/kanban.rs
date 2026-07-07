use gpm_core::AppState;
use gpm_core::domain::kanban::KanbanState;
use gpm_core::infrastructure::github_cli::{self, GhAuthStatus};
use gpm_core::services::kanban::{self, KanbanRefresh};
use tauri::State;

#[tauri::command]
pub fn check_gh_auth() -> GhAuthStatus {
    github_cli::check_auth()
}

#[tauri::command]
pub async fn refresh_kanban(state: State<'_, AppState>) -> Result<KanbanRefresh, String> {
    kanban::refresh(state.inner()).await.map_err(|e| e.to_string())
}

#[tauri::command]
// Tauri's invoke layer hands commands owned State/args; the signature is the command contract.
#[allow(clippy::needless_pass_by_value)]
pub fn load_kanban_local(state: State<AppState>) -> Result<Option<KanbanRefresh>, String> {
    kanban::load_local(state.inner()).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn move_kanban_card(
    state: State<'_, AppState>,
    name_with_owner: String,
    to_column: String,
) -> Result<KanbanState, String> {
    kanban::move_card(state.inner(), &name_with_owner, &to_column)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_github_repo(
    state: State<'_, AppState>,
    name_with_owner: String,
) -> Result<KanbanRefresh, String> {
    kanban::delete_repo(state.inner(), &name_with_owner)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
// Tauri's invoke layer hands commands owned State/args; the signature is the command contract.
#[allow(clippy::needless_pass_by_value)]
pub fn open_url(url: String) -> Result<(), String> {
    gpm_core::infrastructure::launcher::open_url(&url).map_err(|e| e.to_string())
}
