use axum::{
    routing::{delete, get, post, put}, Router
};

use crate::state::AppState;

pub mod list_tables;
pub mod get_db_schema;
pub mod db_list;
pub mod db_query;
pub mod db_update;
pub mod db_add;
pub mod db_delete;
pub mod get_indices;
pub mod build_index;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/",
            get(list_tables::list_tables),
        )
        .route("/{table}", put(db_add::add_to_table).delete(db_delete::delete_from_table).get(db_list::list_items))
        .route("/{table}/index", post(build_index::build_index))
        .route("/{table}/query", post(db_query::query_table))
        .route("/{table}/schema", get(get_db_schema::get_db_schema))
        .route("/{table}/indices", get(get_indices::get_db_indices))
}
