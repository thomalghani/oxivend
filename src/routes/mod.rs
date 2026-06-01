//! HTTP route handlers.

use axum::{
    Router,
    routing::{delete, get, post, put},
};
use std::sync::Arc;

use crate::state::AppState;

pub mod auth;
pub mod health;
pub mod licenses;
pub mod products;

/// Build the application router with all routes attached.
pub fn app_router(state: Arc<AppState>) -> Router {
    let product_routes = Router::new()
        .route("/products", get(products::list_products))
        .route("/products", post(products::post_product))
        .route("/products/{id}", get(products::get_product))
        .route("/products/{id}", put(products::put_product))
        .route("/products/{id}", delete(products::delete_product))
        .route("/products/{id}/keys", post(products::generate_keys));

    let license_routes = Router::new().route("/licenses", post(licenses::post_license));

    Router::new()
        .route("/health", get(health::health))
        .route("/login", post(auth::login))
        .route("/auth/refresh", post(auth::refresh))
        .merge(product_routes)
        .merge(license_routes)
        .with_state(state)
}
