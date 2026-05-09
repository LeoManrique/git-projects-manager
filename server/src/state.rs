use crate::db::DbPool;
use crate::google::GoogleVerifier;

#[derive(Clone)]
pub struct AppState {
    pub db: DbPool,
    pub google: GoogleVerifier,
    pub session_ttl_secs: i64,
}
