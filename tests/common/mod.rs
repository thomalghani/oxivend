//! Shared setup for API integration tests.
#![allow(dead_code)]

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use serde_json::Value;
use tower::ServiceExt;

use oxivend::{auth, config::Config, db, routes, state::AppState};

/// Build a fully configured app router and return it with the config.
pub async fn setup() -> (axum::Router, Config) {
    dotenvy::dotenv().ok();
    let config = Config::from_env().expect("failed to load config");
    let pool = db::create_pool(&config.database_url, config.database_pool_size)
        .await
        .expect("failed to create pool");
    db::run_migrations(&pool)
        .await
        .expect("failed to run migrations");
    auth::ensure_admin(&pool, &config.admin_email, &config.admin_password)
        .await
        .expect("failed to seed admin");

    let state = Arc::new(AppState {
        pool,
        jwt_secret: config.jwt_secret.clone(),
        private_key_encryption_key: config.private_key_encryption_key.clone(),
        access_token_ttl_seconds: config.access_token_ttl_seconds,
        refresh_token_ttl_days: config.refresh_token_ttl_days,
        refresh_token_bytes: config.refresh_token_bytes,
    });

    (routes::app_router(state), config)
}

/// Login with the seeded admin credentials and return the access token.
pub async fn login(router: &axum::Router) -> String {
    let email = std::env::var("OXIVEND_ADMIN_EMAIL").unwrap_or_else(|_| "admin@example.com".into());
    let password = std::env::var("OXIVEND_ADMIN_PASSWORD").unwrap_or_else(|_| "changeme".into());

    let body = serde_json::json!({ "email": email, "password": password });
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/login")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&body).unwrap()))
                .unwrap(),
        )
        .await
        .expect("login request failed");

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("failed to read body");
    let json: Value = serde_json::from_slice(&bytes).expect("failed to parse JSON");
    json["access_token"].as_str().unwrap().to_string()
}

/// Make an authenticated GET request.
pub async fn get(router: &axum::Router, uri: &str, token: &str) -> (StatusCode, Value) {
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(uri)
                .header("authorization", format!("Bearer {token}"))
                .body(Body::from(String::new()))
                .unwrap(),
        )
        .await
        .expect("request failed");
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("failed to read body");
    let json: Value = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
    (status, json)
}

/// Make an authenticated POST request.
pub async fn post(
    router: &axum::Router,
    uri: &str,
    token: Option<&str>,
    body: &serde_json::Value,
) -> (StatusCode, Value) {
    let mut builder = Request::builder()
        .method(Method::POST)
        .uri(uri)
        .header("content-type", "application/json");
    if let Some(t) = token {
        builder = builder.header("authorization", format!("Bearer {t}"));
    }
    let response = router
        .clone()
        .oneshot(
            builder
                .body(Body::from(serde_json::to_string(body).unwrap()))
                .unwrap(),
        )
        .await
        .expect("request failed");
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("failed to read body");
    let json: Value = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
    (status, json)
}

/// Make an authenticated PUT request.
pub async fn put(
    router: &axum::Router,
    uri: &str,
    token: &str,
    body: &serde_json::Value,
) -> (StatusCode, Value) {
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::PUT)
                .uri(uri)
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::from(serde_json::to_string(body).unwrap()))
                .unwrap(),
        )
        .await
        .expect("request failed");
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("failed to read body");
    let json: Value = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
    (status, json)
}

/// Make an authenticated DELETE request.
pub async fn delete(router: &axum::Router, uri: &str, token: &str) -> (StatusCode, Value) {
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::DELETE)
                .uri(uri)
                .header("authorization", format!("Bearer {token}"))
                .body(Body::from(String::new()))
                .unwrap(),
        )
        .await
        .expect("request failed");
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("failed to read body");
    let json: Value = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
    (status, json)
}
