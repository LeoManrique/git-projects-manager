//! The one data folder both frontends share:
//! `dirs::config_dir()/git-projects-manager/`.

use anyhow::{Context, Result};
use std::path::PathBuf;

const APP_DIR_NAME: &str = "git-projects-manager";

/// The app's data folder, created if it does not exist yet.
///
/// # Errors
/// Returns an error if the platform config directory cannot be determined or
/// the app folder cannot be created.
pub fn app_data_dir() -> Result<PathBuf> {
    let dir = dirs::config_dir()
        .context("Could not find config directory")?
        .join(APP_DIR_NAME);
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}
