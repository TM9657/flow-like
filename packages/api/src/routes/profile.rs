use crate::state::AppState;
use axum::{
    Router,
    routing::{get, post},
};

pub mod create_default;
pub mod delete_profile;
pub mod get_profiles;
pub mod sync_profiles;
pub mod upsert_profile;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(get_profiles::get_profiles))
        .route("/sync", post(sync_profiles::sync_profiles))
        .route(
            "/{profile_id}",
            post(upsert_profile::upsert_profile).delete(delete_profile::delete_profile),
        )
}
