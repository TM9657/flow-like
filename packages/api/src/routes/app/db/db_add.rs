use crate::{
    ensure_permission, error::ApiError, middleware::jwt::AppUser,
    permission::role_permission::RolePermissions, state::AppState,
};
use axum::{
    Extension, Json,
    extract::{Path, State},
};
use flow_like_storage::databases::vector::{lancedb::LanceDBVectorStore, VectorStore};
use flow_like_types::anyhow;
use futures_util::{StreamExt, TryStreamExt};

#[derive(Debug, Clone, serde::Deserialize)]
pub struct AddToDBPayload {
    pub items: Vec<flow_like_types::Value>,
}

#[tracing::instrument(name = "PUT /apps/{app_id}/db/{table}", skip(state, user))]
pub async fn add_to_table(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Path((app_id, table)): Path<(String, String)>,
    Json(payload): Json<AddToDBPayload>,
) -> Result<Json<()>, ApiError> {
    ensure_permission!(user, &app_id, &state, RolePermissions::WriteFiles);

    let credentials = state.master_credentials().await?;
    let connection = credentials.to_db(&app_id).await?.execute().await?;
    let mut db = LanceDBVectorStore::from_connection(connection, table).await;

    db.insert(payload.items).await?;

    Ok(Json(()))
}
