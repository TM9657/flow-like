use crate::{
    ensure_permission, error::ApiError, middleware::jwt::AppUser,
    permission::role_permission::RolePermissions, state::AppState,
};
use axum::{
    Extension, Json,
    extract::{Path, State},
};
use flow_like::flow::{
    board::{Board, ExecutionMode, ExecutionStage},
    execution::LogLevel,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize)]
pub struct UpsertBoard {
    pub name: Option<String>,
    pub description: Option<String>,
    pub stage: Option<ExecutionStage>,
    pub log_level: Option<LogLevel>,
    pub execution_mode: Option<ExecutionMode>,
    pub template: Option<Board>,
}

#[derive(Deserialize, Serialize)]
pub struct UpsertBoardResponse {
    pub id: String,
}

#[tracing::instrument(
    name = "PUT /apps/{app_id}/board/{board_id}",
    skip(state, user, params)
)]
pub async fn upsert_board(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Path((app_id, board_id)): Path<(String, String)>,
    Json(params): Json<UpsertBoard>,
) -> Result<Json<UpsertBoardResponse>, ApiError> {
    let permission = ensure_permission!(user, &app_id, &state, RolePermissions::WriteBoards);
    let sub = permission.sub()?;

    let mut app = state.master_app(&sub, &app_id, &state).await?;
    let mut id = board_id.clone();
    if !app.boards.contains(&board_id) {
        id = app.create_board(None, params.template).await?;
        app.save().await?;
    }

    let board = app.open_board(id, Some(false), None).await?;
    let mut board = board.lock().await;
    board.name = params.name.unwrap_or(board.name.clone());
    board.description = params.description.unwrap_or(board.description.clone());
    board.stage = params.stage.unwrap_or(board.stage.clone());
    board.log_level = params.log_level.unwrap_or(board.log_level);
    board.execution_mode = params
        .execution_mode
        .unwrap_or(board.execution_mode.clone());
    board.save(None).await?;

    Ok(Json(UpsertBoardResponse {
        id: board.id.clone(),
    }))
}
