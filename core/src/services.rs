//! Frontend-agnostic orchestration flows shared by the Tauri command layer
//! and the macOS `UniFFI` bridge, so neither frontend re-implements them.

pub mod auth;
pub mod kanban;
