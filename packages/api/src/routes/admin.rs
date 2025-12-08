use axum::{
    Router,
    routing::{get, patch, put},
};
use bit::{delete_bit, push_meta, upsert_bit};

use crate::state::AppState;

pub mod bit;
pub mod profiles;
pub mod solutions;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/bit/{bit_id}",
            put(upsert_bit::upsert_bit).delete(delete_bit::delete_bit),
        )
        .route("/bit/{bit_id}/{language}", put(push_meta::push_meta))
        .route(
            "/profiles/media",
            get(profiles::get_signed_profile_img_url::get_signed_profile_img_url),
        )
        .route(
            "/profiles/{profile_id}",
            put(profiles::upsert_profile_template::upsert_profile_template)
                .delete(profiles::delete_profile_template::delete_profile_template),
        )
        .route("/solutions", get(solutions::list_solutions::list_solutions))
        .route(
            "/solutions/{solution_id}",
            get(solutions::get_solution::get_solution)
                .patch(solutions::update_solution::update_solution),
        )
}
