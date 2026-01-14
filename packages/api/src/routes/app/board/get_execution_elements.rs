use crate::{
    ensure_permission, error::ApiError, middleware::jwt::AppUser,
    permission::role_permission::RolePermissions, state::AppState,
};
use axum::{
    Extension, Json,
    extract::{Path, Query, State},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use flow_like_types::Value;

#[derive(Debug, Deserialize)]
pub struct GetExecutionElementsQuery {
    pub page_id: String,
    #[serde(default)]
    pub wildcard: bool,
}

#[derive(Serialize)]
pub struct GetExecutionElementsResponse {
    pub elements: HashMap<String, Value>,
}

/// Gets the elements required for executing a workflow on a specific page.
///
/// This returns only the elements that are referenced by nodes in the board,
/// along with their children. Use `wildcard: true` to get all elements.
#[tracing::instrument(name = "GET /apps/{app_id}/board/{board_id}/elements", skip(state, user))]
pub async fn get_execution_elements(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Path((app_id, board_id)): Path<(String, String)>,
    Query(query): Query<GetExecutionElementsQuery>,
) -> Result<Json<GetExecutionElementsResponse>, ApiError> {
    let permission = ensure_permission!(user, &app_id, &state, RolePermissions::ExecuteBoards);
    let sub = permission.sub()?;

    let board = state
        .master_board(&sub, &app_id, &board_id, &state, None)
        .await?;

    let elements = board.get_execution_elements(&query.page_id, query.wildcard, None).await?;

    Ok(Json(GetExecutionElementsResponse { elements }))
}
