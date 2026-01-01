use axum::{Json, Router, http::StatusCode, routing::get};
use serde::Serialize;

#[derive(Serialize)]
pub struct HealthResponse {
    status: String,
    version: String,
}

pub fn routes() -> Router {
    Router::new()
        .route("/live", get(liveness))
        .route("/ready", get(readiness))
        .route("/startup", get(startup))
}

async fn liveness() -> (StatusCode, Json<HealthResponse>) {
    (
        StatusCode::OK,
        Json(HealthResponse {
            status: "healthy".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }),
    )
}

async fn readiness() -> (StatusCode, Json<HealthResponse>) {
    (
        StatusCode::OK,
        Json(HealthResponse {
            status: "ready".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }),
    )
}

async fn startup() -> (StatusCode, Json<HealthResponse>) {
    (
        StatusCode::OK,
        Json(HealthResponse {
            status: "started".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }),
    )
}
