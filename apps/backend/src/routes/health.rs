use crate::error::AppError;
use crate::state::AppState;
use axum::extract::State;
use axum::Json;
use axum::{routing::get, Router};
use serde_json::json;
use std::time::Instant;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(|| async { "ok" }))
        .route("/db", get(db_state_handler))
}

#[tracing::instrument]
async fn db_state_handler(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let state = state.db.clone();
    let now = Instant::now();
    state.ping().await?;
    let elapsed = now.elapsed();
    let response = Json(json!({
        "rtt": elapsed.as_millis()
    }));
    Ok(response)
}
