use std::sync::Arc;

use oxivend::{auth, config::Config, db, routes, state::AppState};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    dotenvy::dotenv().ok();

    let config = Config::from_env()?;
    let pool = db::create_pool(&config.database_url, config.database_pool_size).await?;
    db::run_migrations(&pool).await?;
    auth::ensure_admin(&pool, &config.admin_email, &config.admin_password).await?;

    let state = Arc::new(AppState {
        pool,
        jwt_secret: config.jwt_secret,
        private_key_encryption_key: config.private_key_encryption_key,
        access_token_ttl_seconds: config.access_token_ttl_seconds,
        refresh_token_ttl_days: config.refresh_token_ttl_days,
        refresh_token_bytes: config.refresh_token_bytes,
    });

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    tracing::info!("Listening on 0.0.0.0:3000");
    axum::serve(listener, routes::app_router(state)).await?;

    Ok(())
}
