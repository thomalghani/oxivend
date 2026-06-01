//! Integration tests for auth endpoints (login, refresh).

mod common;

use axum::http::StatusCode;

#[tokio::test]
async fn login_with_valid_credentials() {
    let (router, _) = common::setup().await;
    let token = common::login(&router).await;
    assert!(!token.is_empty(), "access token should not be empty");
}

#[tokio::test]
async fn login_with_wrong_password() {
    let (router, _) = common::setup().await;

    let body = serde_json::json!({
        "email": "admin@example.com",
        "password": "wrong-password"
    });
    let (status, json) = common::post(&router, "/login", None, &body).await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(json["error"], "Invalid email or password");
}

#[tokio::test]
async fn login_with_unknown_email() {
    let (router, _) = common::setup().await;

    let body = serde_json::json!({
        "email": "unknown@example.com",
        "password": "some-password"
    });
    let (status, json) = common::post(&router, "/login", None, &body).await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(json["error"], "Invalid email or password");
}

#[tokio::test]
async fn refresh_token_roundtrip() {
    let (router, _) = common::setup().await;

    // Login first
    let email = std::env::var("OXIVEND_ADMIN_EMAIL").unwrap_or_else(|_| "admin@example.com".into());
    let password =
        std::env::var("OXIVEND_ADMIN_PASSWORD").unwrap_or_else(|_| "changeme".into());
    let login_body = serde_json::json!({ "email": email, "password": password });
    let (login_status, login_json) = common::post(&router, "/login", None, &login_body).await;
    assert_eq!(login_status, StatusCode::OK);

    let refresh_token = login_json["refresh_token"].as_str().unwrap().to_string();

    // Use refresh token to get a new pair
    let refresh_body = serde_json::json!({ "refresh_token": refresh_token });
    let (refresh_status, refresh_json) =
        common::post(&router, "/auth/refresh", None, &refresh_body).await;

    assert_eq!(refresh_status, StatusCode::OK);
    assert!(!refresh_json["access_token"].as_str().unwrap().is_empty());
    assert!(!refresh_json["refresh_token"].as_str().unwrap().is_empty());

    // Old refresh token should be consumed (one-time use)
    let (second_status, _) =
        common::post(&router, "/auth/refresh", None, &refresh_body).await;
    assert_eq!(
        second_status,
        StatusCode::UNAUTHORIZED,
        "reused refresh token should be rejected"
    );
}

#[tokio::test]
async fn refresh_with_invalid_token() {
    let (router, _) = common::setup().await;

    let body = serde_json::json!({ "refresh_token": "invalid-token-here" });
    let (status, _) = common::post(&router, "/auth/refresh", None, &body).await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn protected_endpoint_without_auth() {
    let (router, _) = common::setup().await;

    let (status, _) = common::get(&router, "/products", "").await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
}
