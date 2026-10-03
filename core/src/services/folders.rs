//! Monitored-folder changes that also concern the scanner.

use crate::AppState;
use anyhow::Result;
use std::path::Path;

/// Update a monitored folder. A changed path drops the scan state of the old
/// one, which no folder monitors any more.
///
/// # Errors
/// Returns an error if no folder has the given `id`, if the new path overlaps
/// another monitored folder, or if the config cannot be loaded or saved.
pub fn update(
    state: &AppState,
    id: String,
    path: String,
    name: String,
    only_local_checks: bool,
    detect_uninitialized: bool,
) -> Result<()> {
    let replaced_path = state.config_manager.update_folder(
        id,
        path,
        name,
        only_local_checks,
        detect_uninitialized,
    )?;
    if let Some(old_path) = replaced_path {
        state.scanner.forget(Path::new(&old_path));
    }
    Ok(())
}

/// Stop monitoring a folder and drop its scan state.
///
/// # Errors
/// Returns an error if the config cannot be loaded or saved.
pub fn delete(state: &AppState, id: String) -> Result<()> {
    if let Some(removed) = state.config_manager.delete_folder(id)? {
        state.scanner.forget(Path::new(&removed.path));
    }
    Ok(())
}
