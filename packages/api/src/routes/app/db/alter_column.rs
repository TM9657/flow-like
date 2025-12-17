use crate::{
    ensure_permission, error::ApiError, middleware::jwt::AppUser,
    permission::role_permission::RolePermissions, state::AppState,
};
use axum::{
    Extension, Json,
    extract::{Path, State},
};
use flow_like_storage::databases::vector::lancedb::LanceDBVectorStore;
use flow_like_storage::lancedb::table::ColumnAlteration;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct AlterColumnPayload {
    pub column: String,
    pub rename: Option<String>,
    pub nullable: Option<bool>,
}

#[tracing::instrument(name = "PUT /apps/{app_id}/db/{table}/columns", skip(state, user))]
pub async fn alter_column(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Path((app_id, table)): Path<(String, String)>,
    Json(payload): Json<AlterColumnPayload>,
) -> Result<Json<()>, ApiError> {
    ensure_permission!(user, &app_id, &state, RolePermissions::WriteFiles);

    let credentials = state.master_credentials().await?;
    let connection = credentials.to_db(&app_id).await?.execute().await?;
    let db = LanceDBVectorStore::from_connection(connection, table).await;

    let mut alteration = ColumnAlteration::new(payload.column.clone());

    if let Some(new_name) = payload.rename {
        alteration = alteration.rename(new_name);
    }

    if let Some(nullable) = payload.nullable {
        alteration = alteration.set_nullable(nullable);
    }

    db.alter_column(&[alteration]).await?;

    Ok(Json(()))
}
