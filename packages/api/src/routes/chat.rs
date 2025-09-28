use axum::{routing::{get, post}, Router};

use crate::state::AppState;

pub mod completions;
pub mod usage;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/completions", post(completions::invoke_llm))
        .route("/usage", get(usage::get_llm_usage))
}
