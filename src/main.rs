use axum::{
    Router,
    routing::{get, post},
};
use std::sync::Arc;

use oxivend::config::Config;
use oxivend::db;
use oxivend::routes;
use oxivend::state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    dotenvy::dotenv().ok();

    let config = Config::from_env()?;

    let pool = db::create_pool(&config.database_url, config.database_pool_size).await?;
    db::run_migrations(&pool).await?;
    oxivend::auth::ensure_admin(&pool, &config.admin_email, &config.admin_password).await?;

    let state = Arc::new(AppState {
        pool,
        jwt_secret: config.jwt_secret,
        access_token_ttl_seconds: config.access_token_ttl_seconds,
        refresh_token_ttl_days: config.refresh_token_ttl_days,
        refresh_token_bytes: config.refresh_token_bytes,
    });

    let app = Router::new()
        .route("/health", get(routes::health::health))
        .route("/login", post(routes::auth::login))
        .route("/auth/refresh", post(routes::auth::refresh))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    tracing::info!("Listening on 0.0.0.0:3000");
    axum::serve(listener, app).await?;

    Ok(())
}
