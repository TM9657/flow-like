use axum::{Router, routing::get};

use crate::state::AppState;

mod history;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/llm", get(history::get_llm_history))
        .route("/embeddings", get(history::get_embedding_history))
        .route("/executions", get(history::get_execution_history))
        .route("/summary", get(history::get_usage_summary))
}
