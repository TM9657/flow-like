use crate::{
    ensure_permission, error::ApiError, middleware::jwt::AppUser,
    permission::role_permission::RolePermissions, routes::PaginationParams, state::AppState,
};
use axum::{
    Extension, Json,
    extract::{Path, Query, State},
};
use flow_like_storage::databases::vector::{VectorStore, lancedb::LanceDBVectorStore};

#[tracing::instrument(name = "GET /apps/{app_id}/db/{table}", skip(state, user))]
pub async fn list_items(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Path((app_id, table)): Path<(String, String)>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Vec<flow_like_types::Value>>, ApiError> {
    ensure_permission!(user, &app_id, &state, RolePermissions::ReadFiles);

    let offset = params.offset.unwrap_or(0) as usize;
    let limit = params.limit.unwrap_or(25).min(250) as usize;

    let credentials = state.master_credentials().await?;
    let connection = credentials.to_db(&app_id).await?.execute().await?;
    let db = LanceDBVectorStore::from_connection(connection, table).await;

    let items = db.list(limit, offset).await?;

    Ok(Json(items))
}
