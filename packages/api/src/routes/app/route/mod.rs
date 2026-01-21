use axum::{Router, routing::get};

use crate::state::AppState;

pub mod create_route;
pub mod delete_route;
pub mod get_default_route;
pub mod get_route_by_path;
pub mod get_routes;
pub mod update_route;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/",
            get(get_routes::get_routes).post(create_route::create_route),
        )
        .route("/by-path", get(get_route_by_path::get_route_by_path))
        .route("/default", get(get_default_route::get_default_route))
        .route(
            "/{route_id}",
            get(get_routes::get_routes)
                .put(update_route::update_route)
                .delete(delete_route::delete_route),
        )
}
