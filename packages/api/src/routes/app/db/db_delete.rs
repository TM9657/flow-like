use crate::{
    ensure_permission, error::ApiError, middleware::jwt::AppUser,
    permission::role_permission::RolePermissions, state::AppState,
};
use axum::{
    Extension, Json,
    extract::{Path, State},
};
use flow_like_storage::databases::vector::{VectorStore, lancedb::LanceDBVectorStore};

#[derive(Debug, Clone, serde::Deserialize)]
pub struct DeleteFromDBPayload {
    pub query: String,
}

#[tracing::instrument(name = "DELETE /apps/{app_id}/db/{table}", skip(state, user))]
pub async fn delete_from_table(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Path((app_id, table)): Path<(String, String)>,
    Json(payload): Json<DeleteFromDBPayload>,
) -> Result<Json<()>, ApiError> {
    ensure_permission!(user, &app_id, &state, RolePermissions::WriteFiles);

    let credentials = state.master_credentials().await?;
    let connection = credentials.to_db(&app_id).await?.execute().await?;
    let db = LanceDBVectorStore::from_connection(connection, table).await;

    db.delete(&payload.query).await?;

    Ok(Json(()))
}
