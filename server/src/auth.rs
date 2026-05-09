use crate::error::ApiError;
use crate::state::AppState;
use axum::{
    Json,
    extract::{FromRequestParts, State},
    http::{HeaderMap, request::Parts},
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct AuthRequest {
    pub id_token: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub session_token: String,
    pub expires_at: i64,
    pub user: AuthUser,
}

#[derive(Serialize)]
pub struct AuthUser {
    pub sub: String,
    pub email: Option<String>,
    pub name: Option<String>,
}

pub async fn google_signin(
    State(state): State<AppState>,
    Json(body): Json<AuthRequest>,
) -> Result<Json<AuthResponse>, ApiError> {
    let claims = state
        .google
        .verify(&body.id_token)
        .await
        .map_err(|e| ApiError::Unauthorized(format!("id_token rejected: {e}")))?;

    let now = Utc::now().timestamp();
    let expires_at = now + state.session_ttl_secs;
    let token = Uuid::new_v4().to_string();
    let sub = claims.sub.clone();
    let email = claims.email.clone();
    let name = claims.name.clone();

    let conn = state.db.get()?;
    conn.execute(
        "INSERT INTO users (sub, email, name, created_at) VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(sub) DO UPDATE SET email = excluded.email, name = excluded.name",
        rusqlite::params![sub, email, name, now],
    )?;
    conn.execute(
        "INSERT INTO sessions (token, sub, created_at, expires_at) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![token, sub, now, expires_at],
    )?;

    Ok(Json(AuthResponse {
        session_token: token,
        expires_at,
        user: AuthUser { sub, email, name },
    }))
}

pub struct Authed {
    pub sub: String,
}

#[axum::async_trait]
impl FromRequestParts<AppState> for Authed {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let token = bearer_token(&parts.headers)
            .ok_or_else(|| ApiError::Unauthorized("missing bearer token".into()))?;
        let now = Utc::now().timestamp();
        let conn = state.db.get()?;
        let sub: Option<String> = conn
            .query_row(
                "SELECT sub FROM sessions WHERE token = ?1 AND expires_at > ?2",
                rusqlite::params![token, now],
                |r| r.get(0),
            )
            .ok();
        let sub = sub.ok_or_else(|| ApiError::Unauthorized("invalid session".into()))?;
        Ok(Authed { sub })
    }
}

fn bearer_token(headers: &HeaderMap) -> Option<String> {
    let h = headers.get(axum::http::header::AUTHORIZATION)?.to_str().ok()?;
    h.strip_prefix("Bearer ").map(|s| s.to_string())
}

pub async fn sign_out(State(state): State<AppState>, authed: Authed) -> Result<(), ApiError> {
    let conn = state.db.get()?;
    conn.execute("DELETE FROM sessions WHERE sub = ?1", rusqlite::params![authed.sub])?;
    Ok(())
}
