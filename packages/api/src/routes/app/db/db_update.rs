use crate::{
    ensure_permission, error::ApiError, middleware::jwt::AppUser,
    permission::role_permission::RolePermissions, state::AppState,
};
use axum::{
    Extension, Json,
    extract::{Path, State},
};
use flow_like_storage::databases::vector::lancedb::LanceDBVectorStore;
use std::collections::HashMap;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct UpdatePayload {
    pub filter: String,
    pub updates: HashMap<String, flow_like_types::Value>,
}

#[tracing::instrument(name = "PUT /apps/{app_id}/db/{table}/update", skip(state, user))]
pub async fn update_table(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Path((app_id, table)): Path<(String, String)>,
    Json(payload): Json<UpdatePayload>,
) -> Result<Json<()>, ApiError> {
    ensure_permission!(user, &app_id, &state, RolePermissions::WriteFiles);

    let credentials = state.master_credentials().await?;
    let connection = credentials.to_db(&app_id).await?.execute().await?;
    let db = LanceDBVectorStore::from_connection(connection, table).await;

    db.update(&payload.filter, payload.updates).await?;

    Ok(Json(()))
}
