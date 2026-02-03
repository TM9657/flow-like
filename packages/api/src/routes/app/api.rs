use crate::state::AppState;
use axum::{
    Router,
    routing::{delete, put},
};

mod create_api_key;
mod delete_api_key;
mod get_api_keys;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", put(create_api_key::create_api_key).get(get_api_keys::get_api_keys))
        .route("/{key_id}", delete(delete_api_key::delete_api_key))
}
