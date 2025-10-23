pub mod delete_board;
pub mod execute_board;
pub mod execute_commands;
pub mod get_board;
pub mod get_board_versions;
pub mod get_boards;
pub mod undo_redo_board;
pub mod upsert_board;
pub mod version_board;
pub mod realtime;

use axum::{
    Router,
    routing::{get, patch},
};

use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(get_boards::get_boards))
        .route(
            "/{board_id}",
            get(get_board::get_board)
                .post(execute_commands::execute_commands)
                .patch(version_board::version_board)
                .put(upsert_board::upsert_board)
                .delete(delete_board::delete_board),
        )
        .route(
            "/{board_id}/version",
            get(get_board_versions::get_board_versions),
        )
        .route(
            "/{board_id}/realtime",
            get(realtime::jwks).post(realtime::access),
        )
        .route("/{board_id}/undo", patch(undo_redo_board::undo_board))
        .route("/{board_id}/redo", patch(undo_redo_board::redo_board))
}
