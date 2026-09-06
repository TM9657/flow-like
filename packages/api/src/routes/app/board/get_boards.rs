use crate::{
    ensure_permission, error::ApiError, middleware::jwt::AppUser,
    permission::role_permission::RolePermissions,
    routes::app::board::secrets::filter_board_secrets, state::AppState,
};
use axum::{
    Extension, Json,
    extract::{Path, State},
};
use flow_like::flow::board::Board;

#[utoipa::path(
    get,
    path = "/apps/{app_id}/board",
    tag = "boards",
    params(
        ("app_id" = String, Path, description = "Application ID")
    ),
    responses(
        (status = 200, description = "List of boards in the application", body = Vec<Object>),
        (status = 401, description = "Unauthorized")
    )
)]
#[tracing::instrument(name = "GET /apps/{app_id}/board", skip(state, user))]
pub async fn get_boards(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Path(app_id): Path<String>,
) -> Result<Json<Vec<Board>>, ApiError> {
    let permission = ensure_permission!(user, &app_id, &state, RolePermissions::ReadBoards);
    let sub = permission.sub()?;

    let mut boards = vec![];

    let app = state.master_app(&sub, &app_id, &state).await?;
    for board_id in app.boards.iter() {
        let board = match app.open_board(board_id.clone(), Some(false), None).await {
            Ok(board) => board,
            Err(error) => {
                if let Some(error) = ApiError::from_board_format_error(&error) {
                    return Err(error);
                }
                continue;
            }
        };
        let mut board = board.lock().await.clone();
        filter_board_secrets(&mut board);
        boards.push(board);
    }

    Ok(Json(boards))
}
