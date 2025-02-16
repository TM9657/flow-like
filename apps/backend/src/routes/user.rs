use axum::middleware;
use axum::{routing::get, Router};
use tower::ServiceBuilder;

use crate::state::AppState;

use super::auth::auth_middleware;

pub fn routes() -> Router<AppState> {
    Router::new()
        .layer(ServiceBuilder::new().layer(middleware::from_fn(auth_middleware)))
        .route("/", get(|| async { "Hello, World!" }))
        .route("/info", get(|| async { "Hello, World!" }))
}
