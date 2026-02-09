use axum::{Router, routing::get};

use crate::state::AppState;

pub mod delete_widget;
pub mod get_widget;
pub mod get_widgets;
pub mod upsert_widget;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(get_widgets::get_widgets))
        .route(
            "/{widget_id}",
            get(get_widget::get_widget)
                .put(upsert_widget::upsert_widget)
                .delete(delete_widget::delete_widget),
        )
}
