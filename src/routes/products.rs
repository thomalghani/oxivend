//! Product CRUD route handlers.

use axum::{
    Json,
    extract::{FromRequestParts, Path, State},
    http::{StatusCode, request::Parts},
    response::{IntoResponse, Response},
};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Row};
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::UserInfo;
use crate::routes::auth::AuthErrorResponse;
use crate::state::AppState;

// ─── Models ───────────────────────────────────────────────────────────────────

/// Database row for a product.
#[derive(Debug, sqlx::FromRow)]
pub struct Product {
    pub id: Uuid,
    pub name: String,
    pub product_type: String,
    pub price: i64,
    pub public_key: Option<String>,
    pub encrypted_private_key: Option<String>,
    pub max_activations: i32,
    pub created_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

/// Create / update request body.
#[derive(Debug, Deserialize)]
pub struct ProductRequest {
    pub name: String,
    #[serde(default = "default_product_type")]
    pub product_type: String,
    pub price: i64,
    #[serde(default = "default_max_activations")]
    pub max_activations: i32,
}

fn default_product_type() -> String {
    "software_license".to_string()
}

fn default_max_activations() -> i32 {
    1
}

/// API response — never exposes `encrypted_private_key`.
#[derive(Debug, Serialize)]
pub struct ProductResponse {
    pub id: Uuid,
    pub name: String,
    pub product_type: String,
    pub price: i64,
    pub public_key: Option<String>,
    pub max_activations: i32,
    pub created_at: DateTime<Utc>,
}

impl From<Product> for ProductResponse {
    fn from(p: Product) -> Self {
        Self {
            id: p.id,
            name: p.name,
            product_type: p.product_type,
            price: p.price,
            public_key: p.public_key,
            max_activations: p.max_activations,
            created_at: p.created_at,
        }
    }
}

// ─── Key generation response ──────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct KeyResponse {
    pub product_id: Uuid,
    pub public_key: String,
}

// ─── Handlers ─────────────────────────────────────────────────────────────────

/// POST /products — create a product (no keys yet).
pub async fn post_product(
    _auth: AuthenticatedAdmin,
    State(state): State<Arc<AppState>>,
    Json(body): Json<ProductRequest>,
) -> Result<Json<ProductResponse>, ProductError> {
    let row = sqlx::query(
        "INSERT INTO products (name, product_type, price, max_activations)
         VALUES ($1, $2, $3, $4)
         RETURNING id, name, product_type, price, public_key, encrypted_private_key,
                   max_activations, created_at, deleted_at",
    )
    .bind(&body.name)
    .bind(&body.product_type)
    .bind(body.price)
    .bind(body.max_activations)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| ProductError::internal(e.to_string()))?;

    let product = Product::from_row(&row).map_err(|e| ProductError::internal(e.to_string()))?;
    Ok(Json(product.into()))
}

/// GET /products — list all active products.
pub async fn list_products(
    _auth: AuthenticatedAdmin,
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<ProductResponse>>, ProductError> {
    let rows = sqlx::query(
        "SELECT id, name, product_type, price, public_key, encrypted_private_key,
                max_activations, created_at, deleted_at
         FROM products WHERE deleted_at IS NULL
         ORDER BY created_at DESC",
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|e| ProductError::internal(e.to_string()))?;

    let products: Vec<ProductResponse> = rows
        .iter()
        .filter_map(|row| Product::from_row(row).ok().map(ProductResponse::from))
        .collect();

    Ok(Json(products))
}

/// GET /products/{id} — get a single product.
pub async fn get_product(
    _auth: AuthenticatedAdmin,
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<ProductResponse>, ProductError> {
    let row = sqlx::query(
        "SELECT id, name, product_type, price, public_key, encrypted_private_key,
                max_activations, created_at, deleted_at
         FROM products WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| ProductError::internal(e.to_string()))?
    .ok_or(ProductError::not_found())?;

    let product = Product::from_row(&row).map_err(|e| ProductError::internal(e.to_string()))?;
    Ok(Json(product.into()))
}

/// PUT /products/{id} — update product metadata.
pub async fn put_product(
    _auth: AuthenticatedAdmin,
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(body): Json<ProductRequest>,
) -> Result<Json<ProductResponse>, ProductError> {
    let row = sqlx::query(
        "UPDATE products
         SET name = $1, product_type = $2, price = $3, max_activations = $4
         WHERE id = $5 AND deleted_at IS NULL
         RETURNING id, name, product_type, price, public_key, encrypted_private_key,
                   max_activations, created_at, deleted_at",
    )
    .bind(&body.name)
    .bind(&body.product_type)
    .bind(body.price)
    .bind(body.max_activations)
    .bind(id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| ProductError::internal(e.to_string()))?
    .ok_or(ProductError::not_found())?;

    let product = Product::from_row(&row).map_err(|e| ProductError::internal(e.to_string()))?;
    Ok(Json(product.into()))
}

/// DELETE /products/{id} — soft-delete a product.
pub async fn delete_product(
    _auth: AuthenticatedAdmin,
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ProductError> {
    let affected =
        sqlx::query("UPDATE products SET deleted_at = NOW() WHERE id = $1 AND deleted_at IS NULL")
            .bind(id)
            .execute(&state.pool)
            .await
            .map_err(|e| ProductError::internal(e.to_string()))?
            .rows_affected();

    if affected == 0 {
        return Err(ProductError::not_found());
    }

    Ok(StatusCode::NO_CONTENT)
}

/// POST /products/{id}/keys — generate Ed25519 key pair for a product.
pub async fn generate_keys(
    _auth: AuthenticatedAdmin,
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<KeyResponse>, ProductError> {
    // Verify product exists and is not deleted
    let row = sqlx::query_scalar::<_, Option<bool>>(
        "SELECT EXISTS(SELECT 1 FROM products WHERE id = $1 AND deleted_at IS NULL)",
    )
    .bind(id)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| ProductError::internal(e.to_string()))?;

    if !row.unwrap_or(false) {
        return Err(ProductError::not_found());
    }

    // Generate Ed25519 key pair
    let signing_key = crate::crypto::generate_keypair();
    let verifying_key = crate::crypto::public_key(&signing_key);

    let private_key_bytes = signing_key.to_keypair_bytes();
    let encrypted =
        crate::crypto::encrypt_private_key(&private_key_bytes, &state.private_key_encryption_key)
            .map_err(|e| ProductError::internal(e.to_string()))?;

    let public_key_b64 = BASE64.encode(verifying_key.to_bytes());

    sqlx::query(
        "UPDATE products SET public_key = $1, encrypted_private_key = $2 WHERE id = $3 AND deleted_at IS NULL",
    )
    .bind(&public_key_b64)
    .bind(&encrypted)
    .bind(id)
    .execute(&state.pool)
    .await
    .map_err(|e| ProductError::internal(e.to_string()))?;

    Ok(Json(KeyResponse {
        product_id: id,
        public_key: public_key_b64,
    }))
}

// ─── Auth extractor for product routes ────────────────────────────────────────

/// Lightweight auth guard — validates JWT, extracts user info.
pub struct AuthenticatedAdmin {
    pub user: UserInfo,
}

impl FromRequestParts<Arc<AppState>> for AuthenticatedAdmin {
    type Rejection = AuthErrorResponse;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        let auth = crate::routes::auth::AuthenticatedUser::from_request_parts(parts, state).await?;

        // Fetch user info from DB (minimal query)
        let row = sqlx::query("SELECT id, name, email FROM users WHERE id = $1")
            .bind(auth.id)
            .fetch_optional(&state.pool)
            .await
            .map_err(|_| AuthErrorResponse::internal())?
            .ok_or(AuthErrorResponse::invalid_token())?;

        let user = UserInfo {
            id: row.get("id"),
            name: row.get("name"),
            email: row.get("email"),
        };

        Ok(Self { user })
    }
}

// ─── Error type ───────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct ProductError {
    pub error: String,
    #[serde(skip)]
    pub status: StatusCode,
}

impl ProductError {
    fn internal(msg: String) -> Self {
        Self {
            error: msg,
            status: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn not_found() -> Self {
        Self {
            error: String::new(),
            status: StatusCode::NOT_FOUND,
        }
    }
}

impl IntoResponse for ProductError {
    fn into_response(self) -> Response {
        let body = if self.status == StatusCode::NOT_FOUND {
            Json(serde_json::json!({"error": "Product not found"}))
        } else {
            Json(serde_json::json!({"error": self.error}))
        };
        (self.status, body).into_response()
    }
}
