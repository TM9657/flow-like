use crate::error::InternalError;
use crate::state::AppState;
use axum::Json;
use axum::extract::State;
use axum::{Router, routing::get};
use serde_json::json;
use std::time::Instant;

pub fn routes() -> Router<AppState> {
    let router = Router::new();

    router
        .route("/", get(|| async { "ok" }))
        .route("/db", get(db_state_handler))
}

async fn db_state_handler(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, InternalError> {
    let state = state.db.clone();
    let now = Instant::now();
    state.ping().await?;
    let elapsed = now.elapsed();
    let response = Json(json!({
        "rtt": elapsed.as_millis()
    }));
    Ok(response)
}
