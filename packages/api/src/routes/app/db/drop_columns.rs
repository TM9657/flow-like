use crate::{
    ensure_permission, error::ApiError, middleware::jwt::AppUser,
    permission::role_permission::RolePermissions, state::AppState,
};
use axum::{
    Extension, Json,
    extract::{Path, State},
};
use flow_like_storage::databases::vector::lancedb::LanceDBVectorStore;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct DropColumnsPayload {
    pub columns: Vec<String>,
}

#[tracing::instrument(name = "DELETE /apps/{app_id}/db/{table}/columns", skip(state, user))]
pub async fn drop_columns(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Path((app_id, table)): Path<(String, String)>,
    Json(payload): Json<DropColumnsPayload>,
) -> Result<Json<()>, ApiError> {
    ensure_permission!(user, &app_id, &state, RolePermissions::WriteFiles);

    let credentials = state.master_credentials().await?;
    let connection = credentials.to_db(&app_id).await?.execute().await?;
    let db = LanceDBVectorStore::from_connection(connection, table).await;

    let column_refs: Vec<&str> = payload.columns.iter().map(|s| s.as_str()).collect();
    db.drop_columns(&column_refs).await?;

    Ok(Json(()))
}
