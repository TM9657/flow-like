use crate::error::InternalError;
use crate::state::AppState;
use axum::Json;
use axum::extract::State;
use axum::{Router, routing::get};
use serde::{Deserialize, Serialize};
use std::time::Instant;
use utoipa::ToSchema;

#[derive(Clone, Serialize, Deserialize, Debug, ToSchema)]
pub struct DbStateResponse {
    pub rtt: u128,
}

pub fn routes() -> Router<AppState> {
    let router = Router::new();

    router
        .route("/", get(|| async { "ok" }))
        .route("/db", get(get_store_db))
}

#[utoipa::path(
    get,
    path = "/store/db",
    tag = "store",
    responses(
        (status = 200, description = "Database connection status", body = DbStateResponse),
        (status = 500, description = "Database connection failed")
    )
)]
pub async fn get_store_db(
    State(state): State<AppState>,
) -> Result<Json<DbStateResponse>, InternalError> {
    let db = state.db.clone();
    let now = Instant::now();
    db.ping().await?;
    let elapsed = now.elapsed();
    let response = Json(DbStateResponse {
        rtt: elapsed.as_millis(),
    });
    Ok(response)
}
