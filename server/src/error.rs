use axum::{Json, http::StatusCode, response::IntoResponse};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("unauthorized: {0}")]
    Unauthorized(String),
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("internal: {0}")]
    Internal(#[from] anyhow::Error),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let (status, msg) = match &self {
            ApiError::Unauthorized(m) => (StatusCode::UNAUTHORIZED, m.clone()),
            ApiError::BadRequest(m) => (StatusCode::BAD_REQUEST, m.clone()),
            ApiError::Internal(e) => {
                tracing::error!(error = ?e, "internal server error");
                (StatusCode::INTERNAL_SERVER_ERROR, "internal".to_string())
            }
        };
        (status, Json(json!({ "error": msg }))).into_response()
    }
}

impl From<rusqlite::Error> for ApiError {
    fn from(e: rusqlite::Error) -> Self {
        ApiError::Internal(e.into())
    }
}

impl From<r2d2::Error> for ApiError {
    fn from(e: r2d2::Error) -> Self {
        ApiError::Internal(e.into())
    }
}
