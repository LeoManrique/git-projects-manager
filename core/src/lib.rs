//! Tauri-free core for Git Projects Manager.
//!
//! Owns all domain logic (git repository scanning, folder management,
//! settings, kanban state, sync) and persistence. Consumed by:
//! - `desktop/src-tauri` — the Tauri app (Windows/Linux).
//! - `macos/ffi` — the `UniFFI` bridge for the native `SwiftUI` app (macOS).
//!
//! Both frontends share the same on-disk stores under
//! `dirs::config_dir()/git-projects-manager/`.

pub mod config;
pub mod domain;
pub mod infrastructure;
mod state;

pub use state::AppState;
