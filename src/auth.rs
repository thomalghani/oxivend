//! Authentication utilities: password hashing, session tokens, first-boot seed.

use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use chrono::Utc;
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AuthError;

// ─── Password ─────────────────────────────────────────────────────────────────

/// Hash a password using Argon2.
pub fn hash_password(password: &str) -> Result<String, AuthError> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| AuthError::Internal(e.to_string()))?;
    Ok(hash.to_string())
}

/// Verify a password against an Argon2 hash.
pub fn verify_password(password: &str, hash: &str) -> Result<bool, AuthError> {
    let parsed_hash = PasswordHash::new(hash).map_err(|e| AuthError::Internal(e.to_string()))?;
    let argon2 = Argon2::default();
    Ok(argon2
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

// ─── Session (JWT) ────────────────────────────────────────────────────────────

/// JWT claims for admin session tokens.
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    /// User ID (UUID as string).
    pub sub: String,
    /// Expiration timestamp (UTC epoch seconds).
    pub exp: usize,
    /// Issued-at timestamp (UTC epoch seconds).
    pub iat: usize,
}

/// Create a signed JWT session token for the given user.
pub fn create_token(user_id: Uuid, secret: &str) -> Result<String, AuthError> {
    let now = Utc::now();
    let claims = Claims {
        sub: user_id.to_string(),
        iat: now.timestamp() as usize,
        exp: (now.timestamp() + 86_400) as usize, // 24 hours
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| AuthError::Internal(e.to_string()))
}

/// Validate a JWT session token and return its claims.
pub fn validate_token(token: &str, secret: &str) -> Result<Claims, AuthError> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|e| {
        tracing::warn!(error = %e, "Failed to decode JWT token");
        AuthError::InvalidToken
    })?;
    Ok(token_data.claims)
}

// ─── First-boot seed ──────────────────────────────────────────────────────────

/// Ensure an admin user exists. If no users are found, create one from the
/// provided email and password. This runs once on first startup.
pub async fn ensure_admin(pool: &PgPool, email: &str, password: &str) -> Result<(), AuthError> {
    let exists = sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM users)")
        .fetch_one(pool)
        .await
        .map_err(|e| AuthError::Internal(e.to_string()))?;

    if exists {
        tracing::info!("Admin user already exists, skipping seed");
        return Ok(());
    }

    let password_hash = hash_password(password)?;
    sqlx::query("INSERT INTO users (name, email, password_hash, role) VALUES ($1, $2, $3, $4)")
        .bind("Admin")
        .bind(email)
        .bind(&password_hash)
        .bind("superadmin")
        .execute(pool)
        .await
        .map_err(|e| AuthError::Internal(e.to_string()))?;

    tracing::info!("Seeded default admin user: {email}");
    Ok(())
}
