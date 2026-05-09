mod auth;
mod db;
mod error;
mod google;
mod state;
mod sync;

use axum::{
    Router,
    routing::{get, post},
};
use std::env;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .init();

    let bind_addr = env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8787".to_string());
    let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| "/data/sync.db".to_string());
    let google_client_id =
        env::var("GOOGLE_CLIENT_ID").map_err(|_| anyhow::anyhow!("GOOGLE_CLIENT_ID required"))?;
    let session_ttl_days: i64 = env::var("SESSION_TTL_DAYS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(60);

    let db = db::init(&database_url)?;
    let state = state::AppState {
        db,
        google: google::GoogleVerifier::new(google_client_id),
        session_ttl_secs: session_ttl_days * 24 * 3600,
    };

    let app = Router::new()
        .route("/v1/health", get(|| async { "ok" }))
        .route("/v1/auth/google", post(auth::google_signin))
        .route("/v1/auth/sign-out", post(auth::sign_out))
        .route("/v1/sync", post(sync::sync))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    tracing::info!("listening on {bind_addr}");
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
