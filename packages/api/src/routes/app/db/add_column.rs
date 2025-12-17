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
pub struct AddColumnPayload {
    pub name: String,
    pub sql_expression: String,
}

#[tracing::instrument(name = "POST /apps/{app_id}/db/{table}/columns", skip(state, user))]
pub async fn add_column(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Path((app_id, table)): Path<(String, String)>,
    Json(payload): Json<AddColumnPayload>,
) -> Result<Json<()>, ApiError> {
    ensure_permission!(user, &app_id, &state, RolePermissions::WriteFiles);

    let credentials = state.master_credentials().await?;
    let connection = credentials.to_db(&app_id).await?.execute().await?;
    let db = LanceDBVectorStore::from_connection(connection, table).await;

    db.add_column(&payload.name, &payload.sql_expression)
        .await?;

    Ok(Json(()))
}
