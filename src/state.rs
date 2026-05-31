//! Shared application state passed to all route handlers via axum's State.

use sqlx::PgPool;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub jwt_secret: String,
    pub access_token_ttl_seconds: i64,
    pub refresh_token_ttl_days: i64,
    pub refresh_token_bytes: usize,
}
