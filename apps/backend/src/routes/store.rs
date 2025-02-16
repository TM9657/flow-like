use crate::error::AppError;
use crate::state::AppState;
use axum::extract::State;
use axum::{middleware, Json};
use axum::{routing::get, Router};
use serde_json::json;
use std::time::Instant;
use tower::ServiceBuilder;

use super::auth::auth_middleware;

pub fn routes(state: &AppState) -> Router<AppState> {
    let mut router = Router::new();

    if !state.platform_config.features.unauthorized_read {
        router = router.layer(ServiceBuilder::new().layer(middleware::from_fn(auth_middleware)));
    }

    router
        .route("/", get(|| async { "ok" }))
        .route("/db", get(db_state_handler))
}

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
