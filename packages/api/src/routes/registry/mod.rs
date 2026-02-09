//! WASM Package Registry API

pub mod download;
mod index;
pub mod publish;
pub mod search;
pub mod server;

use crate::state::AppState;
use axum::{
    Router,
    routing::{get, post},
};

pub use server::ServerRegistry;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/index.json", get(index::index))
        .route("/search", get(search::search))
        .route("/download", post(download::download))
        .route("/publish", post(publish::publish))
        .route("/package/{id}", get(index::get_package))
        .route("/package/{id}/versions", get(index::get_versions))
}
