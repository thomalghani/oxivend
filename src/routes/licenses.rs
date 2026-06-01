//! License issuance route handler.

use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use chrono::{DateTime, Utc};
use ed25519_dalek::SigningKey;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::sync::Arc;
use uuid::Uuid;

use crate::routes::products::AuthenticatedAdmin;
use crate::state::AppState;

// ─── Types ────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct LicenseRequest {
    pub product_id: Uuid,
    pub user_email: String,
    #[serde(default)]
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct LicenseResponse {
    pub id: Uuid,
    pub product_id: Uuid,
    pub user_email: String,
    pub license_token: String,
    pub status: String,
    pub issued_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

// ─── Handler ───────────────────────────────────────────────────────────────────

/// POST /licenses — manually issue a license for a product.
pub async fn post_license(
    _auth: AuthenticatedAdmin,
    State(state): State<Arc<AppState>>,
    Json(body): Json<LicenseRequest>,
) -> Result<Json<LicenseResponse>, LicenseError> {
    // Fetch product, verify it exists and has keys
    let row = sqlx::query(
        "SELECT id, public_key, encrypted_private_key FROM products
         WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(body.product_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| LicenseError::internal(e.to_string()))?
    .ok_or(LicenseError::not_found())?;

    let product_id: Uuid = row.get("id");
    let public_key: Option<String> = row.get("public_key");
    let encrypted_key: Option<String> = row.get("encrypted_private_key");

    let encrypted_key = encrypted_key.ok_or(LicenseError::no_keys())?;
    let _ = public_key.ok_or(LicenseError::no_keys())?;

    // Decrypt private key
    let key_bytes =
        crate::crypto::decrypt_private_key(&encrypted_key, &state.private_key_encryption_key)
            .map_err(|e| LicenseError::internal(e.to_string()))?;

    let key_array: [u8; 64] = key_bytes.try_into().map_err(|e: Vec<u8>| {
        tracing::error!(
            "Invalid decrypted key length: expected 64 bytes, got {}",
            e.len()
        );
        LicenseError::internal("Invalid decrypted key length".into())
    })?;

    let signing_key = SigningKey::from_keypair_bytes(&key_array)
        .map_err(|e| LicenseError::internal(e.to_string()))?;

    // Create license payload and sign it
    let license_id = Uuid::new_v4();
    let now = Utc::now();

    let payload = crate::crypto::LicensePayload {
        format_version: 1,
        algorithm: "ed25519".into(),
        product_id,
        user_email: body.user_email.clone(),
        issued_at: now,
        expires_at: body.expires_at,
        license_id,
    };

    let license_token = crate::crypto::sign_license(&signing_key, &payload)
        .map_err(|e| LicenseError::internal(e.to_string()))?;

    // Store in DB
    sqlx::query(
        "INSERT INTO licenses (id, product_id, user_email, license_token, status, issued_at, expires_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(license_id)
    .bind(product_id)
    .bind(&body.user_email)
    .bind(&license_token)
    .bind("active")
    .bind(now)
    .bind(body.expires_at)
    .execute(&state.pool)
    .await
    .map_err(|e| LicenseError::internal(e.to_string()))?;

    Ok(Json(LicenseResponse {
        id: license_id,
        product_id,
        user_email: body.user_email,
        license_token,
        status: "active".into(),
        issued_at: now,
        expires_at: body.expires_at,
    }))
}

// ─── Error type ────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct LicenseError {
    pub error: String,
    #[serde(skip)]
    pub status: StatusCode,
}

impl LicenseError {
    fn internal(msg: String) -> Self {
        Self {
            error: msg,
            status: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn not_found() -> Self {
        Self {
            error: "Product not found".into(),
            status: StatusCode::NOT_FOUND,
        }
    }

    fn no_keys() -> Self {
        Self {
            error: "Product has no encryption keys. Generate keys first.".into(),
            status: StatusCode::BAD_REQUEST,
        }
    }
}

impl IntoResponse for LicenseError {
    fn into_response(self) -> Response {
        let body = Json(serde_json::json!({"error": self.error}));
        (self.status, body).into_response()
    }
}
