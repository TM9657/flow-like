use crate::{
    ensure_permission, error::ApiError, middleware::jwt::AppUser,
    permission::role_permission::RolePermissions, state::AppState,
};
use axum::{
    Extension, Json,
    extract::{Path, State},
};
use flow_like::flow::board::commands::GenericCommand;
use flow_like_types::anyhow;
use serde::Deserialize;

#[derive(Clone, Deserialize)]
pub struct ExecuteCommandsBody {
    pub commands: Vec<GenericCommand>,
}

#[tracing::instrument(
    name = "PATCH /apps/{app_id}/board/{board_id}/undo",
    skip(state, user, params)
)]
pub async fn undo_board(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Path((app_id, board_id)): Path<(String, String)>,
    Json(params): Json<ExecuteCommandsBody>,
) -> Result<Json<()>, ApiError> {
    let permission = ensure_permission!(user, &app_id, &state, RolePermissions::WriteBoards);
    let sub = permission.sub()?;

    let mut board = state
        .master_board(&sub, &app_id, &board_id, &state, None)
        .await?;

    let flow_state = board
        .app_state
        .clone()
        .ok_or(ApiError::internal_error(anyhow!(
            "No app state found for board"
        )))?
        .clone();

    board.undo(params.commands, flow_state.clone()).await?;
    board.save(None).await?;

    Ok(Json(()))
}

#[tracing::instrument(
    name = "PATCH /apps/{app_id}/board/{board_id}/redo",
    skip(state, user, params)
)]
pub async fn redo_board(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Path((app_id, board_id)): Path<(String, String)>,
    Json(params): Json<ExecuteCommandsBody>,
) -> Result<Json<()>, ApiError> {
    let permission = ensure_permission!(user, &app_id, &state, RolePermissions::WriteBoards);
    let sub = permission.sub()?;

    let mut board = state
        .master_board(&sub, &app_id, &board_id, &state, None)
        .await?;

    let flow_state = board
        .app_state
        .clone()
        .ok_or(ApiError::internal_error(anyhow!(
            "No app state found for board"
        )))?
        .clone();

    board.redo(params.commands, flow_state.clone()).await?;
    board.save(None).await?;

    Ok(Json(()))
}
