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
    pub access_token_ttl_seconds: i64,
    pub refresh_token_ttl_days: i64,
    pub refresh_token_bytes: usize,
}

impl Config {
    /// Load configuration from environment variables.
    ///
    /// Required vars: `DATABASE_URL`, `OXIVEND_ADMIN_EMAIL`, `OXIVEND_ADMIN_PASSWORD`.
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
            access_token_ttl_seconds: std::env::var("ACCESS_TOKEN_TTL_SECONDS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(900),
            refresh_token_ttl_days: std::env::var("REFRESH_TOKEN_TTL_DAYS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(7),
            refresh_token_bytes: std::env::var("REFRESH_TOKEN_BYTES")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(32),
        })
    }
}
