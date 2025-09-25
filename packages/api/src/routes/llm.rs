use axum::{routing::get, Router};

use crate::state::AppState;

pub mod invoke;
pub mod usage;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(usage::get_llm_usage).post(invoke::invoke_llm))
}
