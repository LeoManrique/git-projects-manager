//! `UniFFI` bridge for the native `SwiftUI` macOS app.
//!
//! Exposes the folders/scan/settings/kanban/sync surface of `gpm-core`.
//! Types are mirrored into `UniFFI` records so the core crate stays
//! bindgen-free; orchestration lives in `gpm_core::services`, shared with
//! the Tauri app.

use gpm_core::AppState;
use gpm_core::domain;
use gpm_core::infrastructure::{github_cli, launcher, logging};
use gpm_core::services;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::watch;

uniffi::setup_scaffolding!();

// ---------------------------------------------------------------------------
// Error
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error, uniffi::Error)]
#[uniffi(flat_error)]
pub enum GpmError {
    #[error("{0}")]
    Failure(String),
}

impl From<anyhow::Error> for GpmError {
    fn from(e: anyhow::Error) -> Self {
        Self::Failure(format!("{e:#}"))
    }
}

type FfiResult<T> = Result<T, GpmError>;

// ---------------------------------------------------------------------------
// Records (mirrors of gpm-core domain types)
// ---------------------------------------------------------------------------

#[derive(uniffi::Record)]
pub struct MonitoredFolder {
    pub id: String,
    pub path: String,
    pub name: String,
    pub only_local_checks: bool,
    pub detect_uninitialized: bool,
}

impl From<domain::MonitoredFolder> for MonitoredFolder {
    fn from(f: domain::MonitoredFolder) -> Self {
        Self {
            id: f.id,
            path: f.path,
            name: f.name,
            only_local_checks: f.only_local_checks,
            detect_uninitialized: f.detect_uninitialized,
        }
    }
}

#[derive(uniffi::Enum)]
pub enum PublishState {
    Published,
    Unpublished,
    RemoteNotFound,
}

impl From<domain::PublishState> for PublishState {
    fn from(p: domain::PublishState) -> Self {
        match p {
            domain::PublishState::Published => Self::Published,
            domain::PublishState::Unpublished => Self::Unpublished,
            domain::PublishState::RemoteNotFound => Self::RemoteNotFound,
        }
    }
}

#[derive(uniffi::Record)]
pub struct RepoStatus {
    pub path: String,
    pub branch: Option<String>,
    pub has_changes: Option<bool>,
    pub has_unpushed: Option<bool>,
    pub has_unpulled: Option<bool>,
    pub remote_state_unknown: bool,
    pub publish_state: PublishState,
    pub has_error: bool,
    pub error_message: Option<String>,
}

impl From<domain::RepoStatus> for RepoStatus {
    fn from(r: domain::RepoStatus) -> Self {
        Self {
            path: r.path,
            branch: r.branch,
            has_changes: r.has_changes,
            has_unpushed: r.has_unpushed,
            has_unpulled: r.has_unpulled,
            remote_state_unknown: r.remote_state_unknown,
            publish_state: r.publish_state.into(),
            has_error: r.has_error,
            error_message: r.error_message,
        }
    }
}

#[derive(uniffi::Record)]
pub struct ScanResult {
    pub scanned_path: String,
    pub total_repositories: u32,
    pub started_at_ms: i64,
    pub revision: u64,
    pub pending: Vec<String>,
    pub checking: Vec<RepoStatus>,
    pub is_complete: bool,
    pub with_changes: Vec<RepoStatus>,
    pub with_unpushed: Vec<RepoStatus>,
    pub with_unpulled: Vec<RepoStatus>,
    pub unpublished: Vec<RepoStatus>,
    pub remote_not_found: Vec<RepoStatus>,
    pub remote_state_unknown: Vec<RepoStatus>,
    pub clean: Vec<RepoStatus>,
    pub errors: Vec<RepoStatus>,
    pub uninitialized: Vec<RepoStatus>,
    pub execution_time: f64,
}

impl From<domain::ScanResult> for ScanResult {
    fn from(s: domain::ScanResult) -> Self {
        let map = |v: Vec<domain::RepoStatus>| v.into_iter().map(RepoStatus::from).collect();
        Self {
            scanned_path: s.scanned_path,
            total_repositories: u32::try_from(s.total_repositories).unwrap_or(u32::MAX),
            started_at_ms: s.started_at_ms,
            revision: s.revision,
            pending: s.pending,
            checking: map(s.checking),
            is_complete: s.is_complete,
            with_changes: map(s.with_changes),
            with_unpushed: map(s.with_unpushed),
            with_unpulled: map(s.with_unpulled),
            unpublished: map(s.unpublished),
            remote_not_found: map(s.remote_not_found),
            remote_state_unknown: map(s.remote_state_unknown),
            clean: map(s.clean),
            errors: map(s.errors),
            uninitialized: map(s.uninitialized),
            execution_time: s.execution_time,
        }
    }
}

#[derive(uniffi::Record)]
pub struct TerminalApp {
    pub id: String,
    pub display_name: String,
}

#[derive(uniffi::Record)]
pub struct EditorApp {
    pub id: String,
    pub display_name: String,
}

#[derive(uniffi::Record)]
pub struct AppSettings {
    pub default_terminal: Option<String>,
    pub default_editor: Option<String>,
}

#[derive(uniffi::Record)]
pub struct GitCleanResult {
    pub files_removed: Vec<String>,
    pub directories_removed: Vec<String>,
}

#[derive(uniffi::Record)]
pub struct GhOwner {
    pub login: String,
}

#[derive(uniffi::Record)]
pub struct GhRepo {
    pub name_with_owner: String,
    pub name: String,
    pub owner: GhOwner,
    pub description: Option<String>,
    pub url: String,
    pub is_private: bool,
    pub is_archived: bool,
    pub pushed_at: Option<String>,
}

impl From<github_cli::GhRepo> for GhRepo {
    fn from(r: github_cli::GhRepo) -> Self {
        Self {
            name_with_owner: r.name_with_owner,
            name: r.name,
            owner: GhOwner { login: r.owner.login },
            description: r.description,
            url: r.url,
            is_private: r.is_private,
            is_archived: r.is_archived,
            pushed_at: r.pushed_at,
        }
    }
}

#[derive(uniffi::Enum)]
pub enum GhAuthStatus {
    Ok { user: String },
    NotInstalled,
    NotAuthenticated,
    Error { message: String },
}

impl From<github_cli::GhAuthStatus> for GhAuthStatus {
    fn from(s: github_cli::GhAuthStatus) -> Self {
        match s {
            github_cli::GhAuthStatus::Ok { user } => Self::Ok { user },
            github_cli::GhAuthStatus::NotInstalled => Self::NotInstalled,
            github_cli::GhAuthStatus::NotAuthenticated => Self::NotAuthenticated,
            github_cli::GhAuthStatus::Error { message } => Self::Error { message },
        }
    }
}

#[derive(uniffi::Record)]
pub struct KanbanCard {
    pub name_with_owner: String,
    pub column: String,
    /// `String?` on the Swift side; `None` when the card has no notes.
    pub notes: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl From<domain::kanban::KanbanCard> for KanbanCard {
    fn from(c: domain::kanban::KanbanCard) -> Self {
        Self {
            name_with_owner: c.name_with_owner,
            column: c.column,
            notes: c.notes,
            created_at: c.created_at,
            updated_at: c.updated_at,
        }
    }
}

#[derive(uniffi::Record)]
pub struct KanbanState {
    pub version: u32,
    pub cards: HashMap<String, KanbanCard>,
}

impl From<domain::kanban::KanbanState> for KanbanState {
    fn from(s: domain::kanban::KanbanState) -> Self {
        Self {
            version: s.version,
            cards: s.cards.into_iter().map(|(k, v)| (k, v.into())).collect(),
        }
    }
}

#[derive(uniffi::Enum)]
pub enum SyncStatus {
    Disabled,
    Synced,
    Offline,
    Expired,
}

impl From<domain::auth::SyncStatus> for SyncStatus {
    fn from(s: domain::auth::SyncStatus) -> Self {
        match s {
            domain::auth::SyncStatus::Disabled => Self::Disabled,
            domain::auth::SyncStatus::Synced => Self::Synced,
            domain::auth::SyncStatus::Offline => Self::Offline,
            domain::auth::SyncStatus::Expired => Self::Expired,
        }
    }
}

#[derive(uniffi::Record)]
pub struct SyncUser {
    pub sub: String,
    pub email: Option<String>,
    pub name: Option<String>,
}

impl From<domain::auth::SyncUser> for SyncUser {
    fn from(u: domain::auth::SyncUser) -> Self {
        Self { sub: u.sub, email: u.email, name: u.name }
    }
}

#[derive(uniffi::Record)]
pub struct KanbanRefresh {
    pub repos: Vec<GhRepo>,
    pub state: KanbanState,
    pub sync_status: SyncStatus,
}

impl From<services::kanban::KanbanRefresh> for KanbanRefresh {
    fn from(r: services::kanban::KanbanRefresh) -> Self {
        Self {
            repos: r.repos.into_iter().map(GhRepo::from).collect(),
            state: r.state.into(),
            sync_status: r.sync_status.into(),
        }
    }
}

// ---------------------------------------------------------------------------
// Logging (free functions: the log must start before `GpmCore::new`, so that a
// startup failure is recorded too)
// ---------------------------------------------------------------------------

#[derive(uniffi::Enum)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
}

impl From<LogLevel> for logging::LogLevel {
    fn from(l: LogLevel) -> Self {
        match l {
            LogLevel::Error => Self::Error,
            LogLevel::Warn => Self::Warn,
            LogLevel::Info => Self::Info,
        }
    }
}

/// Start the diagnostics log (`macos.<date>.log`), returning its folder.
/// Idempotent.
///
/// # Errors
/// Errs when the logs folder or today's file cannot be created.
#[uniffi::export]
// UniFFI exports take owned values; the signature is the FFI contract.
#[allow(clippy::needless_pass_by_value)]
pub fn init_logging(app_version: String) -> FfiResult<String> {
    let dir = logging::init("macos", &app_version)?;
    Ok(dir.display().to_string())
}

/// Record a line from the Swift side: an error it showed or swallowed.
#[uniffi::export]
// UniFFI exports take owned values; the signature is the FFI contract.
#[allow(clippy::needless_pass_by_value)]
pub fn log_message(level: LogLevel, message: String) {
    logging::frontend(level.into(), &message);
}

/// Where the log files live.
///
/// # Errors
/// Errs when the app data folder cannot be determined.
#[uniffi::export]
pub fn logs_folder() -> FfiResult<String> {
    Ok(logging::logs_dir()?.display().to_string())
}

/// Open the logs folder in Finder.
///
/// # Errors
/// Errs when the folder cannot be created or Finder fails to launch.
#[uniffi::export]
pub fn open_logs_folder() -> FfiResult<()> {
    logging::open_logs_folder()?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Folder scan handle
// ---------------------------------------------------------------------------

/// One folder scan in flight, read with [`FolderScan::next`] until it returns
/// `None`.
///
/// A watch channel holds only the newest snapshot, so a caller that reads
/// slower than repos land skips the ones in between instead of queueing them.
/// Chosen over a Swift callback, which would run Swift code on the scan's
/// worker threads.
#[derive(uniffi::Object)]
pub struct FolderScan {
    snapshots: tokio::sync::Mutex<watch::Receiver<Option<domain::ScanResult>>>,
}

#[uniffi::export(async_runtime = "tokio")]
impl FolderScan {
    /// Wait for a snapshot newer than the last one returned. `None` once the
    /// final snapshot has been returned, or if the scan died without one.
    pub async fn next(&self) -> Option<ScanResult> {
        let mut snapshots = self.snapshots.lock().await;
        // Errs only when the scan is over and its last snapshot already seen.
        snapshots.changed().await.ok()?;
        snapshots.borrow_and_update().clone().map(ScanResult::from)
    }
}

// ---------------------------------------------------------------------------
// Core object
// ---------------------------------------------------------------------------

#[derive(uniffi::Object)]
pub struct GpmCore {
    state: AppState,
}

#[uniffi::export]
impl GpmCore {
    /// # Errors
    /// Errs when the config directory cannot be created or read.
    #[uniffi::constructor]
    pub fn new() -> FfiResult<Arc<Self>> {
        let state = AppState::new()?;
        Ok(Arc::new(Self { state }))
    }

    // -- Folders ------------------------------------------------------------

    /// # Errors
    /// Errs when `config.json` cannot be read or parsed.
    pub fn get_monitored_folders(&self) -> FfiResult<Vec<MonitoredFolder>> {
        let folders = self.state.config_manager.get_folders()?;
        Ok(folders.into_iter().map(MonitoredFolder::from).collect())
    }

    /// # Errors
    /// Errs when `config.json` cannot be written.
    pub fn add_monitored_folder(
        &self,
        path: String,
        name: String,
        only_local_checks: bool,
        detect_uninitialized: bool,
    ) -> FfiResult<MonitoredFolder> {
        let folder = self.state.config_manager.add_folder(
            path,
            name,
            only_local_checks,
            detect_uninitialized,
        )?;
        Ok(folder.into())
    }

    /// # Errors
    /// Errs when the folder id is unknown or `config.json` cannot be written.
    pub fn update_monitored_folder(
        &self,
        id: String,
        path: String,
        name: String,
        only_local_checks: bool,
        detect_uninitialized: bool,
    ) -> FfiResult<()> {
        services::folders::update(
            &self.state,
            id,
            path,
            name,
            only_local_checks,
            detect_uninitialized,
        )?;
        Ok(())
    }

    /// # Errors
    /// Errs when `config.json` cannot be written.
    pub fn delete_monitored_folder(&self, id: String) -> FfiResult<()> {
        services::folders::delete(&self.state, id)?;
        Ok(())
    }

    // -- Settings -------------------------------------------------------------

    /// # Errors
    /// Errs when `settings.json` cannot be read or parsed.
    pub fn get_app_settings(&self) -> FfiResult<AppSettings> {
        let s = self.state.settings_manager.load()?;
        Ok(AppSettings { default_terminal: s.default_terminal, default_editor: s.default_editor })
    }

    /// # Errors
    /// Errs when `settings.json` cannot be written.
    pub fn set_default_terminal(&self, terminal_id: Option<String>) -> FfiResult<()> {
        self.state.settings_manager.set_default_terminal(terminal_id)?;
        Ok(())
    }

    /// # Errors
    /// Errs when `settings.json` cannot be written.
    pub fn set_default_editor(&self, editor_id: Option<String>) -> FfiResult<()> {
        self.state.settings_manager.set_default_editor(editor_id)?;
        Ok(())
    }

    #[must_use]
    pub fn get_available_terminals(&self) -> Vec<TerminalApp> {
        self.state
            .settings_manager
            .get_available_terminals()
            .into_iter()
            .map(|t| TerminalApp { id: t.id, display_name: t.display_name })
            .collect()
    }

    #[must_use]
    pub fn get_available_editors(&self) -> Vec<EditorApp> {
        self.state
            .settings_manager
            .get_available_editors()
            .into_iter()
            .map(|e| EditorApp { id: e.id, display_name: e.display_name })
            .collect()
    }

    #[must_use]
    pub fn get_git_clean_patterns(&self) -> Vec<String> {
        self.state.settings_manager.get_git_clean_settings().exclude_patterns
    }

    /// # Errors
    /// Errs when `settings.json` cannot be written.
    pub fn set_git_clean_patterns(&self, patterns: Vec<String>) -> FfiResult<()> {
        self.state
            .settings_manager
            .set_git_clean_settings(domain::GitCleanSettings { exclude_patterns: patterns })?;
        Ok(())
    }

    // -- Open actions ----------------------------------------------------------

    /// # Errors
    /// Errs when the terminal id is unknown or the launch fails.
    // UniFFI exports take owned values; the signature is the FFI contract.
    #[allow(clippy::needless_pass_by_value)]
    pub fn open_in_terminal(&self, path: String, terminal_id: String) -> FfiResult<()> {
        launcher::open_terminal_by_id(&self.state.settings_manager, &terminal_id, &path)?;
        Ok(())
    }

    /// # Errors
    /// Errs when the editor id is unknown or the launch fails.
    // UniFFI exports take owned values; the signature is the FFI contract.
    #[allow(clippy::needless_pass_by_value)]
    pub fn open_in_editor(&self, path: String, editor_id: String) -> FfiResult<()> {
        launcher::open_editor_by_id(&self.state.settings_manager, &editor_id, &path)?;
        Ok(())
    }

    /// # Errors
    /// Errs when the login shell fails to spawn.
    // UniFFI exports take owned values; the signature is the FFI contract.
    #[allow(clippy::needless_pass_by_value)]
    pub fn open_in_lms_github(&self, path: String) -> FfiResult<()> {
        launcher::open_in_lms_github(&path)?;
        Ok(())
    }

    /// Open an http(s) URL in the default browser.
    ///
    /// # Errors
    /// Errs when the URL is not http(s) or the browser fails to launch.
    // UniFFI exports take owned values; the signature is the FFI contract.
    #[allow(clippy::needless_pass_by_value)]
    pub fn open_url(&self, url: String) -> FfiResult<()> {
        launcher::open_url(&url)?;
        Ok(())
    }

    // -- Kanban / sync --------------------------------------------------------

    /// The signed-in sync user, if any (in-memory read; the persisted
    /// session is loaded at startup).
    #[must_use]
    pub fn get_sync_user(&self) -> Option<SyncUser> {
        services::auth::current_user(&self.state).map(SyncUser::from)
    }

    /// Offline-first snapshot of the last kanban refresh, or `None` when no
    /// refresh has ever completed on this machine.
    ///
    /// # Errors
    /// Errs when the cache or kanban store cannot be read or parsed.
    pub fn load_kanban_local(&self) -> FfiResult<Option<KanbanRefresh>> {
        Ok(services::kanban::load_local(&self.state)?.map(KanbanRefresh::from))
    }
}

// Blocking operations exported as Swift `async` on a tokio runtime so they
// never block the caller; the heavy lifting is offloaded to blocking threads.
#[uniffi::export(async_runtime = "tokio")]
impl GpmCore {
    /// Scan one monitored folder for git repositories and their status.
    ///
    /// # Errors
    /// Errs when the scan worker thread panics or is cancelled by runtime
    /// shutdown; per-repo failures are reported inside the result instead.
    pub async fn scan_folder(
        &self,
        path: String,
        only_local_checks: bool,
        detect_uninitialized: bool,
    ) -> FfiResult<ScanResult> {
        let scanner = Arc::clone(&self.state.scanner);
        let result = tokio::task::spawn_blocking(move || {
            scanner.scan_folder(
                Path::new(&path),
                only_local_checks,
                detect_uninitialized,
            )
        })
        .await
        .map_err(|e| GpmError::Failure(format!("scan task failed: {e}")))?;
        Ok(result.into())
    }

    /// Start scanning one monitored folder and return the handle its
    /// snapshots arrive through ([`FolderScan::next`]). Returns at once; the
    /// scan blocks a thread of its own.
    #[must_use]
    pub fn start_folder_scan(
        &self,
        path: String,
        only_local_checks: bool,
        detect_uninitialized: bool,
    ) -> Arc<FolderScan> {
        let scanner = Arc::clone(&self.state.scanner);
        let (tx, rx) = watch::channel(None);
        std::thread::spawn(move || {
            let result = scanner.scan_folder_streaming(
                Path::new(&path),
                only_local_checks,
                detect_uninitialized,
                |snapshot| {
                    tx.send_replace(Some(snapshot));
                },
            );
            tx.send_replace(Some(result));
        });
        Arc::new(FolderScan { snapshots: tokio::sync::Mutex::new(rx) })
    }

    /// Read `repos` of the folder at `folder` again after an action changed
    /// them. `None` when the folder was never scanned.
    ///
    /// # Errors
    /// Errs when the worker thread panics or is cancelled by runtime shutdown.
    pub async fn recheck_repos(
        &self,
        folder: String,
        repos: Vec<String>,
        only_local_checks: bool,
    ) -> FfiResult<Option<ScanResult>> {
        let scanner = Arc::clone(&self.state.scanner);
        let result = tokio::task::spawn_blocking(move || {
            let repos: Vec<PathBuf> = repos.into_iter().map(PathBuf::from).collect();
            scanner.recheck_repos(Path::new(&folder), &repos, only_local_checks)
        })
        .await
        .map_err(|e| GpmError::Failure(format!("recheck task failed: {e}")))?;
        Ok(result.map(ScanResult::from))
    }

    /// `git pull` for the repository at `path`.
    ///
    /// # Errors
    /// Errs when the pull fails (dirty tree, auth, network, ...).
    pub async fn pull_repo(&self, path: String) -> FfiResult<String> {
        let out = tokio::task::spawn_blocking(move || {
            gpm_core::infrastructure::git::GitOperations::pull(Path::new(&path))
        })
        .await
        .map_err(|e| GpmError::Failure(format!("pull task failed: {e}")))??;
        Ok(out)
    }

    /// Remove git-ignored files under `path`, preserving matches of the
    /// configured exclude patterns.
    ///
    /// # Errors
    /// Errs when `git clean` fails or a deletion fails.
    pub async fn clean_repo(&self, path: String) -> FfiResult<GitCleanResult> {
        let patterns = self.state.settings_manager.get_git_clean_settings().exclude_patterns;
        let (files, dirs) = tokio::task::spawn_blocking(move || {
            gpm_core::infrastructure::git::GitOperations::clean(Path::new(&path), &patterns)
        })
        .await
        .map_err(|e| GpmError::Failure(format!("clean task failed: {e}")))??;
        Ok(GitCleanResult { files_removed: files, directories_removed: dirs })
    }

    // -- Kanban / sync --------------------------------------------------------

    /// Check GitHub CLI availability and authentication.
    ///
    /// # Errors
    /// Errs when the check worker thread panics; `gh` problems are reported
    /// inside the returned status instead.
    pub async fn check_gh_auth(&self) -> FfiResult<GhAuthStatus> {
        let status = tokio::task::spawn_blocking(github_cli::check_auth)
            .await
            .map_err(|e| GpmError::Failure(format!("gh auth check task failed: {e}")))?;
        Ok(status.into())
    }

    /// Full board refresh: list GitHub repos, reconcile the local board,
    /// sync with the cloud when signed in, persist the offline cache.
    ///
    /// # Errors
    /// Errs when the `gh` repo listing fails or the kanban store cannot be
    /// read or written; sync failures degrade to an offline/expired status.
    pub async fn refresh_kanban(&self) -> FfiResult<KanbanRefresh> {
        Ok(services::kanban::refresh(&self.state).await?.into())
    }

    /// Move a card to another column; a one-card cloud sync runs in the
    /// background when signed in.
    ///
    /// # Errors
    /// Errs when the kanban store cannot be read or written.
    // UniFFI exports take owned values; the signature is the FFI contract.
    #[allow(clippy::needless_pass_by_value)]
    pub async fn move_kanban_card(
        &self,
        name_with_owner: String,
        to_column: String,
    ) -> FfiResult<KanbanState> {
        Ok(services::kanban::move_card(&self.state, &name_with_owner, &to_column).await?.into())
    }

    /// Replace a card's notes. Core trims them and stores blank text as
    /// none; a one-card cloud sync runs in the background when signed in.
    ///
    /// # Errors
    /// Errs when the kanban store cannot be read or written.
    // UniFFI exports take owned values; the signature is the FFI contract.
    #[allow(clippy::needless_pass_by_value)]
    pub async fn update_kanban_notes(
        &self,
        name_with_owner: String,
        notes: Option<String>,
    ) -> FfiResult<KanbanState> {
        Ok(services::kanban::set_notes(&self.state, &name_with_owner, notes).await?.into())
    }

    /// Permanently delete a repository on GitHub, then rebuild the board.
    ///
    /// # Errors
    /// Errs when the deletion or re-listing fails, or the kanban store
    /// cannot be written.
    // UniFFI exports take owned values; the signature is the FFI contract.
    #[allow(clippy::needless_pass_by_value)]
    pub async fn delete_github_repo(&self, name_with_owner: String) -> FfiResult<KanbanRefresh> {
        Ok(services::kanban::delete_repo(&self.state, &name_with_owner).await?.into())
    }

    /// Google sign-in via the loopback-PKCE browser flow; persists the
    /// resulting sync session.
    ///
    /// # Errors
    /// Errs when the OAuth flow fails or times out, or the sync server
    /// rejects the sign-in.
    pub async fn sign_in_with_google(&self) -> FfiResult<SyncUser> {
        let user = services::auth::sign_in(&self.state, launcher::open_url).await?;
        Ok(user.into())
    }

    /// Best-effort server sign-out; the local session is always cleared.
    pub async fn sign_out(&self) {
        services::auth::sign_out(&self.state).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn folder_scan() -> (watch::Sender<Option<domain::ScanResult>>, FolderScan) {
        let (tx, rx) = watch::channel(None);
        (tx, FolderScan { snapshots: tokio::sync::Mutex::new(rx) })
    }

    fn snapshot(revision: u64) -> domain::ScanResult {
        domain::ScanResult { revision, ..domain::ScanResult::default() }
    }

    fn next_revision(scan: &FolderScan) -> Option<u64> {
        tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap()
            .block_on(scan.next())
            .map(|s| s.revision)
    }

    #[test]
    fn next_skips_to_the_newest_snapshot() {
        let (tx, scan) = folder_scan();
        tx.send_replace(Some(snapshot(1)));
        tx.send_replace(Some(snapshot(2)));

        assert_eq!(next_revision(&scan), Some(2));
    }

    #[test]
    fn next_returns_the_final_snapshot_then_none() {
        let (tx, scan) = folder_scan();
        tx.send_replace(Some(snapshot(3)));
        drop(tx);

        assert_eq!(next_revision(&scan), Some(3));
        assert_eq!(next_revision(&scan), None);
    }
}
