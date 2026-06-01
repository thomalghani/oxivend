//! Authentication utilities: password hashing, session tokens, refresh tokens,
//! and first-boot seed.

use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::errors::AuthError;

// ─── User model ──────────────────────────────────────────────────────────────

/// Minimal user model for authentication.
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub password_hash: String,
}

/// Public-facing user info (no password hash).
#[derive(Debug, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: Uuid,
    pub name: String,
    pub email: String,
}

/// Look up a user by email.
pub async fn find_user_by_email(pool: &PgPool, email: &str) -> Result<Option<User>, AuthError> {
    let user = sqlx::query_as::<_, User>(
        "SELECT id, name, email, password_hash FROM users WHERE email = $1",
    )
    .bind(email)
    .fetch_optional(pool)
    .await
    .map_err(|e| AuthError::Internal(e.to_string()))?;
    Ok(user)
}

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
    pub sub: String,
    pub exp: usize,
    pub iat: usize,
}

/// Create a short-lived JWT access token.
pub fn create_access_token(
    user_id: Uuid,
    secret: &str,
    ttl_seconds: i64,
) -> Result<String, AuthError> {
    let now = Utc::now();
    let claims = Claims {
        sub: user_id.to_string(),
        iat: now.timestamp() as usize,
        exp: (now.timestamp() + ttl_seconds) as usize,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| AuthError::Internal(e.to_string()))
}

/// Validate a JWT access token and return its claims.
pub fn validate_access_token(token: &str, secret: &str) -> Result<Claims, AuthError> {
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

// ─── Refresh Token ────────────────────────────────────────────────────────────

/// Generate a random refresh token, store its SHA-256 hash in the DB,
/// and return the raw token to the client.
pub async fn issue_refresh_token(
    pool: &PgPool,
    user_id: Uuid,
    token_bytes: usize,
    ttl_days: i64,
) -> Result<String, AuthError> {
    let raw: Vec<u8> = (0..token_bytes).map(|_| rand::random::<u8>()).collect();
    let raw_token = hex::encode(&raw);
    let hash = sha256_hex(&raw_token);
    let expires_at = Utc::now() + Duration::days(ttl_days);

    sqlx::query("INSERT INTO refresh_tokens (user_id, token_hash, expires_at) VALUES ($1, $2, $3)")
        .bind(user_id)
        .bind(&hash)
        .bind(expires_at)
        .execute(pool)
        .await
        .map_err(|e| AuthError::Internal(e.to_string()))?;

    Ok(raw_token)
}

/// Consume (look up + revoke) a refresh token. Returns the user_id on success.
/// The token is one-time-use: revoked after successful consumption.
pub async fn consume_refresh_token(pool: &PgPool, raw_token: &str) -> Result<Uuid, AuthError> {
    let hash = sha256_hex(raw_token);

    let record = sqlx::query(
        "SELECT id, user_id FROM refresh_tokens
         WHERE token_hash = $1 AND revoked = FALSE AND expires_at > NOW()",
    )
    .bind(&hash)
    .fetch_optional(pool)
    .await
    .map_err(|e| AuthError::Internal(e.to_string()))?
    .ok_or(AuthError::InvalidToken)?;

    let token_id: Uuid = record.get("id");
    let user_id: Uuid = record.get("user_id");

    // Revoke it (one-time use)
    sqlx::query("UPDATE refresh_tokens SET revoked = TRUE WHERE id = $1")
        .bind(token_id)
        .execute(pool)
        .await
        .map_err(|e| AuthError::Internal(e.to_string()))?;

    Ok(user_id)
}

/// Revoke all refresh tokens for a user (e.g., password change).
pub async fn revoke_user_tokens(pool: &PgPool, user_id: Uuid) -> Result<(), AuthError> {
    sqlx::query("UPDATE refresh_tokens SET revoked = TRUE WHERE user_id = $1 AND revoked = FALSE")
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(|e| AuthError::Internal(e.to_string()))?;
    Ok(())
}

fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}

// ─── First-boot seed ──────────────────────────────────────────────────────────

/// Ensure an admin user exists. If the email is not found, create one from env vars.
/// Idempotent — safe to call concurrently from tests.
pub async fn ensure_admin(pool: &PgPool, email: &str, password: &str) -> Result<(), AuthError> {
    let exists =
        sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM users WHERE email = $1)")
            .bind(email)
            .fetch_one(pool)
            .await
            .map_err(|e| AuthError::Internal(e.to_string()))?;

    if exists {
        tracing::info!("Admin user {email} already exists, skipping seed");
        return Ok(());
    }

    let password_hash = hash_password(password)?;
    sqlx::query(
        "INSERT INTO users (name, email, password_hash, role) VALUES ($1, $2, $3, $4)
         ON CONFLICT (email) DO NOTHING",
    )
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
