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
pub enum IndexType {
    FullText,
    BTree,
    Bitmap,
    LabelList,
    Auto
}

impl IndexType {
    pub fn to_string(&self) -> String {
        match self {
            IndexType::FullText => "FULL TEXT".to_string(),
            IndexType::BTree => "BTREE".to_string(),
            IndexType::Bitmap => "BITMAP".to_string(),
            IndexType::LabelList => "LABEL LIST".to_string(),
            IndexType::Auto => "AUTO".to_string(),
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct BuildIndexPayload {
    pub column: String,
    pub index_type: IndexType,
    pub optimize: bool,
}

#[tracing::instrument(name = "POST /apps/{app_id}/db/{table}/index", skip(state, user))]
pub async fn build_index(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Path((app_id, table)): Path<(String, String)>,
    Json(payload): Json<BuildIndexPayload>,
) -> Result<Json<()>, ApiError> {
    ensure_permission!(user, &app_id, &state, RolePermissions::WriteFiles);

    let credentials = state.master_credentials().await?;
    let connection = credentials.to_db(&app_id).await?.execute().await?;
    let db = LanceDBVectorStore::from_connection(connection, table).await;

    db.index(&payload.column, Some(&payload.index_type.to_string())).await?;

    Ok(Json(()))
}
