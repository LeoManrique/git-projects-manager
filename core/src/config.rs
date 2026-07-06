//! Build-time configuration with hardcoded fallbacks.
//!
//! Each value below is resolved in this order:
//! 1. Compile-time env var (set via `cargo build`'s environment) — wins.
//! 2. Runtime env var — debug builds only, for convenience.
//! 3. The hardcoded fallback constant.

/// GCP "Desktop app" OAuth 2.0 client ID for Google sign-in.
/// Replace before shipping.
const OAUTH_CLIENT_ID_FALLBACK: &str = "926356926853-l6jbhemd4pecslsl2blctnr55cdsrrhc.apps.googleusercontent.com";

/// GCP "Desktop app" OAuth 2.0 client secret. Required by Google's token
/// endpoint even with PKCE. Not actually secret — the binary is distributed
/// publicly, so treat this as a registration identifier, not a credential.
const OAUTH_CLIENT_SECRET_FALLBACK: &str = "GOCSPX-REDACTED";

/// Production sync server URL. Replace before shipping.
const SYNC_SERVER_URL_FALLBACK: &str = "https://gpm-sync.leonardomanrique.com";

/// Default sync server URL for `cargo run` / `pnpm tauri dev`.
const SYNC_SERVER_URL_DEV_DEFAULT: &str = "http://localhost:8787";

#[must_use]
pub fn oauth_client_id() -> &'static str {
    option_env!("GOOGLE_OAUTH_CLIENT_ID").unwrap_or(OAUTH_CLIENT_ID_FALLBACK)
}

#[must_use]
pub fn oauth_client_secret() -> &'static str {
    option_env!("GOOGLE_OAUTH_CLIENT_SECRET").unwrap_or(OAUTH_CLIENT_SECRET_FALLBACK)
}

#[must_use]
pub fn sync_server_url() -> String {
    if let Some(u) = option_env!("SYNC_SERVER_URL") {
        return u.to_string();
    }
    if cfg!(debug_assertions) {
        std::env::var("SYNC_SERVER_URL").unwrap_or_else(|_| SYNC_SERVER_URL_DEV_DEFAULT.to_string())
    } else {
        SYNC_SERVER_URL_FALLBACK.to_string()
    }
}
