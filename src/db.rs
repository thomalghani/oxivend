//! Database connection and pool management.

use sqlx::postgres::{PgPool, PgPoolOptions};

use crate::errors::DbError;

/// Create a new database connection pool.
pub async fn create_pool(database_url: &str, max_connections: u32) -> Result<PgPool, DbError> {
    PgPoolOptions::new()
        .max_connections(max_connections)
        .connect(database_url)
        .await
        .map_err(|e| DbError::ConnectionFailed(e.to_string()))
}

/// Run all pending database migrations.
pub async fn run_migrations(pool: &PgPool) -> Result<(), DbError> {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .map_err(|e| DbError::QueryFailed(e.to_string()))
}
