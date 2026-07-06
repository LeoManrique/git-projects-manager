//! Launching external applications (terminals, editors, browser, helper CLIs)
//! for a repository path. Shared by the Tauri command layer and the macOS FFI.

use crate::domain::{EditorApp, OpenMethod, TerminalApp};
use crate::infrastructure::settings_store::SettingsManager;
use anyhow::{Context, Result, bail};
use std::process::Command;

/// Open `path` in the terminal identified by `terminal_id`.
///
/// # Errors
/// Errs when the id does not match an available terminal or the launch
/// command fails to spawn.
pub fn open_terminal_by_id(settings: &SettingsManager, terminal_id: &str, path: &str) -> Result<()> {
    let terminals = settings.get_available_terminals();
    let terminal = terminals
        .iter()
        .find(|t| t.id == terminal_id)
        .with_context(|| format!("Terminal '{terminal_id}' not found"))?;
    open_in_terminal(terminal, path)
}

/// Open `path` in the editor identified by `editor_id`.
///
/// # Errors
/// Errs when the id does not match an available editor or the launch
/// command fails to spawn.
pub fn open_editor_by_id(settings: &SettingsManager, editor_id: &str, path: &str) -> Result<()> {
    let editors = settings.get_available_editors();
    let editor = editors
        .iter()
        .find(|e| e.id == editor_id)
        .with_context(|| format!("Editor '{editor_id}' not found"))?;
    open_in_editor(editor, path)
}

/// Open `path` in the given terminal using its configured open method.
///
/// # Errors
/// Errs when the underlying `osascript`/`open` command fails to spawn.
pub fn open_in_terminal(terminal: &TerminalApp, path: &str) -> Result<()> {
    match &terminal.open_method {
        OpenMethod::AppleScript { script } => {
            let escaped_path = path.replace('\'', "'\\''");
            let final_script = script.replace("{path}", &escaped_path);
            Command::new("osascript").args(["-e", &final_script]).spawn()?;
        }
        OpenMethod::Open | OpenMethod::Default => {
            Command::new("open").args(["-a", &terminal.name, path]).spawn()?;
        }
    }
    Ok(())
}

/// Open `path` in the given editor (via `open -a`).
///
/// # Errors
/// Errs when the `open` command fails to spawn.
pub fn open_in_editor(editor: &EditorApp, path: &str) -> Result<()> {
    Command::new("open").args(["-a", &editor.name, path]).spawn()?;
    Ok(())
}

/// Open `path` with the `lms-github` helper, resolved through the user's
/// login shell so it inherits the interactive PATH.
///
/// # Errors
/// Errs when the shell fails to spawn.
pub fn open_in_lms_github(path: &str) -> Result<()> {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
    Command::new(shell).args(["-lc", "exec lms-github \"$0\"", path]).spawn()?;
    Ok(())
}

/// Open an `http(s)` URL in the system default browser.
///
/// # Errors
/// Errs when the URL is not http(s) or the platform open command fails to
/// spawn.
pub fn open_url(url: &str) -> Result<()> {
    if !url.starts_with("https://") && !url.starts_with("http://") {
        bail!("refusing to open non-http(s) URL");
    }

    #[cfg(target_os = "macos")]
    Command::new("open").arg(url).spawn()?;

    #[cfg(target_os = "linux")]
    Command::new("xdg-open").arg(url).spawn()?;

    #[cfg(target_os = "windows")]
    Command::new("cmd").args(["/C", "start", "", url]).spawn()?;

    Ok(())
}
