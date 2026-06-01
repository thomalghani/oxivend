//! Integration tests for product CRUD and key generation.

mod common;

use axum::http::StatusCode;

#[tokio::test]
async fn create_product() {
    let (router, _) = common::setup().await;
    let token = common::login(&router).await;

    let body = serde_json::json!({
        "name": "Test Product",
        "product_type": "software_license",
        "price": 1999,
        "max_activations": 3
    });
    let (status, json) = common::post(&router, "/products", Some(&token), &body).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["name"], "Test Product");
    assert_eq!(json["price"], 1999);
    assert_eq!(json["max_activations"], 3);
    assert!(json["id"].as_str().unwrap().len() > 0);
    assert!(json["public_key"].is_null());
}

#[tokio::test]
async fn create_product_without_auth() {
    let (router, _) = common::setup().await;

    let body = serde_json::json!({
        "name": "Unauthorized Product",
        "price": 999
    });
    let (status, _) = common::post(&router, "/products", None, &body).await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn list_products() {
    let (router, _) = common::setup().await;
    let token = common::login(&router).await;

    let product = serde_json::json!({ "name": "Product A", "price": 1000 });
    common::post(&router, "/products", Some(&token), &product).await;

    let product = serde_json::json!({ "name": "Product B", "price": 2000 });
    common::post(&router, "/products", Some(&token), &product).await;

    let (status, json) = common::get(&router, "/products", &token).await;

    assert_eq!(status, StatusCode::OK);
    let arr = json.as_array().unwrap();
    assert!(arr.len() >= 2);
}

#[tokio::test]
async fn get_product_by_id() {
    let (router, _) = common::setup().await;
    let token = common::login(&router).await;

    let body = serde_json::json!({ "name": "Get Me", "price": 500 });
    let (_, create_json) = common::post(&router, "/products", Some(&token), &body).await;
    let id = create_json["id"].as_str().unwrap();

    let (status, json) = common::get(&router, &format!("/products/{id}"), &token).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["name"], "Get Me");
}

#[tokio::test]
async fn get_product_not_found() {
    let (router, _) = common::setup().await;
    let token = common::login(&router).await;

    let (status, _) =
        common::get(&router, "/products/00000000-0000-0000-0000-000000000000", &token).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn update_product() {
    let (router, _) = common::setup().await;
    let token = common::login(&router).await;

    let body = serde_json::json!({ "name": "Original", "price": 1000 });
    let (_, create_json) = common::post(&router, "/products", Some(&token), &body).await;
    let id = create_json["id"].as_str().unwrap();

    let update = serde_json::json!({ "name": "Updated", "price": 2500, "max_activations": 5 });
    let (status, json) =
        common::put(&router, &format!("/products/{id}"), &token, &update).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["name"], "Updated");
    assert_eq!(json["price"], 2500);
    assert_eq!(json["max_activations"], 5);
}

#[tokio::test]
async fn update_product_not_found() {
    let (router, _) = common::setup().await;
    let token = common::login(&router).await;

    let body = serde_json::json!({ "name": "Ghost", "price": 0 });
    let (status, _) = common::put(
        &router,
        "/products/00000000-0000-0000-0000-000000000000",
        &token,
        &body,
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn soft_delete_product() {
    let (router, _) = common::setup().await;
    let token = common::login(&router).await;

    let body = serde_json::json!({ "name": "Delete Me", "price": 999 });
    let (_, create_json) = common::post(&router, "/products", Some(&token), &body).await;
    let id = create_json["id"].as_str().unwrap();

    let (status, _) =
        common::delete(&router, &format!("/products/{id}"), &token).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // Product should no longer be visible
    let (status, _) = common::get(&router, &format!("/products/{id}"), &token).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn delete_product_not_found() {
    let (router, _) = common::setup().await;
    let token = common::login(&router).await;

    let (status, _) =
        common::delete(&router, "/products/00000000-0000-0000-0000-000000000000", &token).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn generate_keys_for_product() {
    let (router, _) = common::setup().await;
    let token = common::login(&router).await;

    let body = serde_json::json!({ "name": "Keyed Product", "price": 1500 });
    let (_, create_json) = common::post(&router, "/products", Some(&token), &body).await;
    let id = create_json["id"].as_str().unwrap();

    let (status, json) =
        common::post(&router, &format!("/products/{id}/keys"), Some(&token), &serde_json::json!({})).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["product_id"], id);
    assert!(!json["public_key"].as_str().unwrap().is_empty());

    // Product should now show a public key
    let (_, get_json) = common::get(&router, &format!("/products/{id}"), &token).await;
    assert_eq!(get_json["public_key"], json["public_key"]);
}

#[tokio::test]
async fn generate_keys_product_not_found() {
    let (router, _) = common::setup().await;
    let token = common::login(&router).await;

    let (status, _) = common::post(
        &router,
        "/products/00000000-0000-0000-0000-000000000000/keys",
        Some(&token),
        &serde_json::json!({}),
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}
