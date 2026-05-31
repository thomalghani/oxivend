use axum::{Router, routing::get};

use oxivend::config::Config;
use oxivend::db;
use oxivend::routes;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    dotenvy::dotenv().ok();

    let config = Config::from_env()?;

    let pool = db::create_pool(&config.database_url, config.database_pool_size).await?;
    db::run_migrations(&pool).await?;
    oxivend::auth::ensure_admin(&pool, &config.admin_email, &config.admin_password).await?;

    let _jwt_secret = config.jwt_secret;

    let app = Router::new().route("/health", get(routes::health::health));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    tracing::info!("Listening on 0.0.0.0:3000");
    axum::serve(listener, app).await?;

    Ok(())
}
