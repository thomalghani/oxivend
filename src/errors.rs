//! Domain error types.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum OxivendError {
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Validation error: {0}")]
    Validation(String),
    #[error("Unauthorized: {0}")]
    Unauthorized(String),
    #[error("Internal error: {0}")]
    Internal(String),
}

#[derive(Error, Debug)]
pub enum CryptoError {
    #[error("Invalid key: {0}")]
    InvalidKey(String),
    #[error("Signature verification failed")]
    VerificationFailed,
}

#[derive(Error, Debug)]
pub enum AuthError {
    #[error("Invalid credentials")]
    InvalidCredentials,
    #[error("Invalid or expired token")]
    InvalidToken,
    #[error("Internal auth error: {0}")]
    Internal(String),
}

#[derive(Error, Debug)]
pub enum DbError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    #[error("Query failed: {0}")]
    QueryFailed(String),
}

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Missing required environment variable: {0}")]
    MissingEnvVar(&'static str),
    #[error("Invalid value for {var}: {detail}")]
    InvalidValue { var: &'static str, detail: String },
}
