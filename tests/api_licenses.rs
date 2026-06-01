//! Integration tests for license issuance.

mod common;

use axum::http::StatusCode;

#[tokio::test]
async fn issue_license() {
    let (router, _) = common::setup().await;
    let token = common::login(&router).await;

    // Create a product
    let body = serde_json::json!({ "name": "Licensed Product", "price": 1999 });
    let (_, product_json) = common::post(&router, "/products", Some(&token), &body).await;
    let product_id = product_json["id"].as_str().unwrap();

    // Generate keys for the product
    let (_, keys_json) = common::post(
        &router,
        &format!("/products/{product_id}/keys"),
        Some(&token),
        &serde_json::json!({}),
    )
    .await;
    assert!(!keys_json["public_key"].as_str().unwrap().is_empty());

    // Issue a license
    let license_body = serde_json::json!({
        "product_id": product_id,
        "user_email": "customer@example.com"
    });
    let (status, json) = common::post(&router, "/licenses", Some(&token), &license_body).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["product_id"], product_id);
    assert_eq!(json["user_email"], "customer@example.com");
    assert_eq!(json["status"], "active");
    assert!(!json["license_token"].as_str().unwrap().is_empty());
    assert!(json["id"].as_str().unwrap().len() > 0);
}

#[tokio::test]
async fn issue_license_with_expiry() {
    let (router, _) = common::setup().await;
    let token = common::login(&router).await;

    let body = serde_json::json!({ "name": "Temporary Product", "price": 999 });
    let (_, product) = common::post(&router, "/products", Some(&token), &body).await;
    let product_id = product["id"].as_str().unwrap();

    common::post(
        &router,
        &format!("/products/{product_id}/keys"),
        Some(&token),
        &serde_json::json!({}),
    )
    .await;

    let license_body = serde_json::json!({
        "product_id": product_id,
        "user_email": "temp@example.com",
        "expires_at": "2027-12-31T23:59:59Z"
    });
    let (status, json) = common::post(&router, "/licenses", Some(&token), &license_body).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["user_email"], "temp@example.com");
    assert!(json["expires_at"].as_str().unwrap().contains("2027"));
}

#[tokio::test]
async fn issue_license_product_not_found() {
    let (router, _) = common::setup().await;
    let token = common::login(&router).await;

    let body = serde_json::json!({
        "product_id": "00000000-0000-0000-0000-000000000000",
        "user_email": "customer@example.com"
    });
    let (status, _) = common::post(&router, "/licenses", Some(&token), &body).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn issue_license_without_keys() {
    let (router, _) = common::setup().await;
    let token = common::login(&router).await;

    // Create a product but don't generate keys
    let body = serde_json::json!({ "name": "No Keys", "price": 500 });
    let (_, product) = common::post(&router, "/products", Some(&token), &body).await;
    let product_id = product["id"].as_str().unwrap();

    let license_body = serde_json::json!({
        "product_id": product_id,
        "user_email": "customer@example.com"
    });
    let (status, json) = common::post(&router, "/licenses", Some(&token), &license_body).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(json["error"].as_str().unwrap().contains("keys"));
}

#[tokio::test]
async fn issue_license_without_auth() {
    let (router, _) = common::setup().await;

    let body = serde_json::json!({
        "product_id": "00000000-0000-0000-0000-000000000000",
        "user_email": "customer@example.com"
    });
    let (status, _) = common::post(&router, "/licenses", None, &body).await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
}
