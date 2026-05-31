//! Application configuration loaded from environment variables.

use crate::errors::ConfigError;

/// Centralized configuration for Oxivend.
///
/// All environment variables are read once here and passed explicitly to
/// the functions that need them. This avoids scattered `std::env::var()`
/// calls throughout the codebase.
#[derive(Debug)]
pub struct Config {
    pub database_url: String,
    pub database_pool_size: u32,
    pub admin_email: String,
    pub admin_password: String,
    pub jwt_secret: String,
}

impl Config {
    /// Load configuration from environment variables.
    ///
    /// Required vars: `DATABASE_URL`, `OXIVEND_ADMIN_EMAIL`, `OXIVEND_ADMIN_PASSWORD`.
    /// Optional vars with defaults: `DATABASE_POOL_SIZE` (10), `JWT_SECRET` (dev default).
    pub fn from_env() -> Result<Self, ConfigError> {
        Ok(Self {
            database_url: std::env::var("DATABASE_URL")
                .map_err(|_| ConfigError::MissingEnvVar("DATABASE_URL"))?,
            database_pool_size: std::env::var("DATABASE_POOL_SIZE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(10),
            admin_email: std::env::var("OXIVEND_ADMIN_EMAIL")
                .map_err(|_| ConfigError::MissingEnvVar("OXIVEND_ADMIN_EMAIL"))?,
            admin_password: std::env::var("OXIVEND_ADMIN_PASSWORD")
                .map_err(|_| ConfigError::MissingEnvVar("OXIVEND_ADMIN_PASSWORD"))?,
            jwt_secret: std::env::var("JWT_SECRET")
                .unwrap_or_else(|_| "changeme-dev-secret".to_string()),
        })
    }
}
