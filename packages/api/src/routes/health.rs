use crate::error::InternalError;
use crate::state::AppState;
use axum::Json;
use axum::extract::State;
use axum::{Router, routing::get};
use flow_like_types::Value;
use serde::{Deserialize, Serialize};
use std::time::Instant;
use utoipa::ToSchema;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(health))
        .route("/db", get(db_health))
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct HealthResponse {
    pub status: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct DbHealthResponse {
    pub rtt: u128,
}

#[utoipa::path(
    get,
    path = "/health",
    tag = "health",
    responses(
        (status = 200, description = "Service is healthy", body = HealthResponse)
    )
)]
#[tracing::instrument(name = "GET /health")]
pub async fn health() -> Result<Json<HealthResponse>, InternalError> {
    Ok(Json(HealthResponse {
        status: "ok".to_string(),
    }))
}

#[utoipa::path(
    get,
    path = "/health/db",
    tag = "health",
    responses(
        (status = 200, description = "Database health check with round-trip time", body = DbHealthResponse)
    )
)]
#[tracing::instrument(name = "GET /health/db", skip(state))]
pub async fn db_health(
    State(state): State<AppState>,
) -> Result<Json<DbHealthResponse>, InternalError> {
    let state = state.db.clone();
    let now = Instant::now();
    state.ping().await?;
    let elapsed = now.elapsed();
    Ok(Json(DbHealthResponse {
        rtt: elapsed.as_millis(),
    }))
}
