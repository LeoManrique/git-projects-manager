//! The diagnostics log both frontends write through the core.
//!
//! Both apps run this core against the same data folder, so their logs live
//! together in `<app data dir>/logs/`: one file per app per day
//! (`macos.2026-09-24.log`, `desktop.2026-09-24.log`), the newest
//! [`MAX_LOG_FILES`] of each kept. A separate prefix per app keeps two apps
//! running at once from interleaving lines in one file, and retention counts
//! each app's files on their own.
//!
//! The core owns the file rather than each frontend's native logger, so git
//! failures, the frontends' own errors and panics land in one place, in one
//! format, with one retention policy.

use crate::infrastructure::app_dir::app_data_dir;
use crate::infrastructure::launcher;
use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::PathBuf;
use std::sync::OnceLock;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::fmt::{self, time::ChronoLocal};
use tracing_subscriber::prelude::*;

/// Days of logs kept per app. Rotation is daily, so this is two weeks.
const MAX_LOG_FILES: usize = 14;

/// The outcome of the one [`init`] that ran, replayed to every later call.
static INIT: OnceLock<Result<PathBuf, String>> = OnceLock::new();

/// Severity of a line the frontend asks the core to log.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Error,
    Warn,
    Info,
}

/// Where the log files live.
///
/// # Errors
/// Returns an error if the app data folder cannot be determined or created.
pub fn logs_dir() -> Result<PathBuf> {
    Ok(app_data_dir()?.join("logs"))
}

/// Start writing the log for `app` (the file prefix: `macos` or `desktop`),
/// returning the folder it writes to.
///
/// Idempotent: only the first call installs anything, and every call returns
/// what that first one did. Call it before anything else in the app, so
/// startup failures are logged too.
///
/// # Errors
/// Returns an error if the logs folder or today's file cannot be created, or
/// another global tracing subscriber was installed first.
pub fn init(app: &str, app_version: &str) -> Result<PathBuf> {
    INIT.get_or_init(|| install(app, app_version).map_err(|e| format!("{e:#}")))
        .clone()
        .map_err(anyhow::Error::msg)
}

fn install(app: &str, app_version: &str) -> Result<PathBuf> {
    let dir = logs_dir()?;
    std::fs::create_dir_all(&dir)?;

    let appender = RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .filename_prefix(app)
        .filename_suffix("log")
        .max_log_files(MAX_LOG_FILES)
        .build(&dir)
        .context("could not open the log file")?;

    // Written straight through, not via `tracing_appender::non_blocking`: the
    // volume is a few lines per git command, and a background writer would
    // need a guard that nothing in an FFI library can own until exit, so the
    // last lines before a crash would be the ones lost.
    let file_layer = fmt::layer()
        .with_writer(appender)
        .with_ansi(false)
        .with_timer(ChronoLocal::rfc_3339());

    // Debug builds also print to stderr: the Xcode console, the `tauri dev`
    // terminal.
    let console_layer = cfg!(debug_assertions).then(|| {
        fmt::layer()
            .with_writer(std::io::stderr)
            .with_timer(ChronoLocal::rfc_3339())
    });

    let subscriber = tracing_subscriber::registry()
        .with(LevelFilter::INFO)
        .with(file_layer)
        .with(console_layer);
    tracing::subscriber::set_global_default(subscriber)
        .context("a tracing subscriber was already installed")?;

    log_panics();
    tracing::info!(app, app_version, "logging started");
    Ok(dir)
}

/// Record every panic before the default hook prints it to a stderr that a
/// GUI app launched from Finder or a desktop entry has nowhere to show.
fn log_panics() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let info_text = log_safe(&info.to_string());
        let backtrace = log_safe(&std::backtrace::Backtrace::force_capture().to_string());
        tracing::error!(info = %info_text, %backtrace, "panic");
        previous(info);
    }));
}

/// Log a line on behalf of the frontend: an error it showed the user, or one
/// it recovered from silently.
pub fn frontend(level: LogLevel, message: &str) {
    let message = log_safe(message);
    match level {
        LogLevel::Error => tracing::error!(target: "frontend", "{message}"),
        LogLevel::Warn => tracing::warn!(target: "frontend", "{message}"),
        LogLevel::Info => tracing::info!(target: "frontend", "{message}"),
    }
}

/// `text` made fit for the log: every piece of free text passes through here.
///
/// * Credentials in URLs are masked (`https://user:token@host` →
///   `https://***@host`). git can echo a remote URL with its embedded token,
///   and the log is a plain file the Logs panel invites users to share.
/// * Trimmed and folded onto one line, so a multi-line value (git's stderr, a
///   banner quoting it, a backtrace) cannot split one entry across lines that
///   read as several.
#[must_use]
pub fn log_safe(text: &str) -> String {
    redact_credentials(text.trim())
        .replace("\r\n", "\n")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

/// Replace the `user[:password]@` part of every `scheme://` URL in `text`.
fn redact_credentials(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("://") {
        let (before, after) = rest.split_at(start + 3);
        out.push_str(before);
        // The authority ends at the first `/`, whitespace or quote; a `@`
        // before that closes the userinfo.
        let authority_end = after
            .find(|c: char| c == '/' || c.is_whitespace() || c == '\'' || c == '"')
            .unwrap_or(after.len());
        match after[..authority_end].rfind('@') {
            Some(at) => {
                out.push_str("***");
                rest = &after[at..];
            }
            None => rest = after,
        }
    }
    out.push_str(rest);
    out
}

/// Open the logs folder in the system file manager.
///
/// # Errors
/// Returns an error if the folder cannot be created or the file manager fails
/// to launch.
pub fn open_logs_folder() -> Result<()> {
    let dir = logs_dir()?;
    std::fs::create_dir_all(&dir)?;
    launcher::open_path(&dir.to_string_lossy())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_safe_folds_git_stderr_onto_one_line() {
        let stderr = "From https://github.com/x/y\r\n * [new tag] v1 -> v1\nfatal: boom\rdone\n";
        assert_eq!(
            log_safe(stderr),
            "From https://github.com/x/y\\n * [new tag] v1 -> v1\\nfatal: boom\\rdone"
        );
    }

    #[test]
    fn log_safe_masks_credentials_in_urls() {
        let stderr = "fatal: unable to access 'https://leo:ghp_secret@github.com/x/y.git/': 403";
        assert_eq!(
            log_safe(stderr),
            "fatal: unable to access 'https://***@github.com/x/y.git/': 403"
        );
    }

    #[test]
    fn log_safe_masks_only_the_userinfo_of_scheme_urls() {
        let text = "https://github.com/x/y and git@github.com:x/y.git and ssh://git@host/r";
        assert_eq!(
            log_safe(text),
            "https://github.com/x/y and git@github.com:x/y.git and ssh://***@host/r"
        );
    }

    #[test]
    fn frontend_levels_parse_from_lowercase() {
        let level: LogLevel = serde_json::from_str("\"warn\"").expect("should parse");
        assert_eq!(level, LogLevel::Warn);
    }
}
