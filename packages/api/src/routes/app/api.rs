use crate::state::AppState;
use axum::{
    Router,
    routing::{delete, put},
};

pub mod create_api_key;
pub mod delete_api_key;
pub mod get_api_keys;

pub use create_api_key::{ApiKeyInput, ApiKeyOut};
pub use get_api_keys::ApiKeyInfo;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/",
            put(create_api_key::create_api_key).get(get_api_keys::get_api_keys),
        )
        .route("/{key_id}", delete(delete_api_key::delete_api_key))
}
