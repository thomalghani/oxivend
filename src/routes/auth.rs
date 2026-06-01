//! Authentication route handlers: login and token refresh.

use axum::{
    Json,
    extract::{FromRequestParts, State},
    http::{StatusCode, request::Parts},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::UserInfo;
use crate::state::AppState;

// ─── Request / Response types ─────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub user: UserInfo,
}

#[derive(Debug, Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Debug, Serialize)]
pub struct RefreshResponse {
    pub access_token: String,
    pub refresh_token: String,
}

// ─── Handlers ─────────────────────────────────────────────────────────────────

/// POST /login
pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(body): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, AuthErrorResponse> {
    let user = crate::auth::find_user_by_email(&state.pool, &body.email)
        .await
        .map_err(|_| AuthErrorResponse::internal())?
        .ok_or(AuthErrorResponse::invalid_credentials())?;

    let valid = crate::auth::verify_password(&body.password, &user.password_hash)
        .map_err(|_| AuthErrorResponse::internal())?;

    if !valid {
        return Err(AuthErrorResponse::invalid_credentials());
    }

    let user_id = user.id;
    let access_token = crate::auth::create_access_token(
        user_id,
        &state.jwt_secret,
        state.access_token_ttl_seconds,
    )
    .map_err(|_| AuthErrorResponse::internal())?;

    let refresh_token = crate::auth::issue_refresh_token(
        &state.pool,
        user_id,
        state.refresh_token_bytes,
        state.refresh_token_ttl_days,
    )
    .await
    .map_err(|_| AuthErrorResponse::internal())?;

    Ok(Json(LoginResponse {
        access_token,
        refresh_token,
        user: UserInfo {
            id: user.id,
            name: user.name,
            email: user.email,
        },
    }))
}

/// POST /auth/refresh
pub async fn refresh(
    State(state): State<Arc<AppState>>,
    Json(body): Json<RefreshRequest>,
) -> Result<Json<RefreshResponse>, AuthErrorResponse> {
    let user_id = crate::auth::consume_refresh_token(&state.pool, &body.refresh_token)
        .await
        .map_err(|_| AuthErrorResponse::invalid_token())?;

    let access_token = crate::auth::create_access_token(
        user_id,
        &state.jwt_secret,
        state.access_token_ttl_seconds,
    )
    .map_err(|_| AuthErrorResponse::internal())?;

    let refresh_token = crate::auth::issue_refresh_token(
        &state.pool,
        user_id,
        state.refresh_token_bytes,
        state.refresh_token_ttl_days,
    )
    .await
    .map_err(|_| AuthErrorResponse::internal())?;

    Ok(Json(RefreshResponse {
        access_token,
        refresh_token,
    }))
}

// ─── AuthenticatedUser extractor ──────────────────────────────────────────────

/// Extractor for protected routes. Validates the JWT from the Authorization header.
pub struct AuthenticatedUser {
    pub id: Uuid,
}

impl FromRequestParts<Arc<AppState>> for AuthenticatedUser {
    type Rejection = AuthErrorResponse;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get("Authorization")
            .and_then(|v| v.to_str().ok())
            .ok_or(AuthErrorResponse::invalid_token())?;

        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or(AuthErrorResponse::invalid_token())?;

        let claims = crate::auth::validate_access_token(token, &state.jwt_secret)
            .map_err(|_| AuthErrorResponse::invalid_token())?;

        let user_id =
            Uuid::parse_str(&claims.sub).map_err(|_| AuthErrorResponse::invalid_token())?;

        Ok(AuthenticatedUser { id: user_id })
    }
}

// ─── Error response ───────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct AuthErrorResponse {
    pub error: &'static str,
    #[serde(skip)]
    pub status: StatusCode,
}

impl AuthErrorResponse {
    pub fn invalid_credentials() -> Self {
        Self {
            error: "Invalid email or password",
            status: StatusCode::UNAUTHORIZED,
        }
    }

    pub fn invalid_token() -> Self {
        Self {
            error: "Invalid or expired token",
            status: StatusCode::UNAUTHORIZED,
        }
    }

    pub fn internal() -> Self {
        Self {
            error: "Internal server error",
            status: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for AuthErrorResponse {
    fn into_response(self) -> Response {
        (self.status, Json(self)).into_response()
    }
}
