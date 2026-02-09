use axum::{Router, routing::get};

use crate::state::AppState;

pub mod delete_page;
pub mod get_page;
pub mod get_page_by_route;
pub mod get_pages;
pub mod upsert_page;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(get_pages::get_pages))
        .route("/by-route", get(get_page_by_route::get_page_by_route))
        .route(
            "/{page_id}",
            get(get_page::get_page)
                .put(upsert_page::upsert_page)
                .delete(delete_page::delete_page),
        )
}
