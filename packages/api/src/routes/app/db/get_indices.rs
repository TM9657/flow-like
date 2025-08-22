use crate::{
    ensure_permission,
    error::ApiError,
    middleware::jwt::AppUser,
    permission::role_permission::RolePermissions,
    routes::{LanguageParams, PaginationParams},
    state::AppState,
};
use axum::{
    Extension, Json,
    extract::{Path, Query, State},
};
use flow_like_storage::{
    arrow_schema::Schema,
    databases::vector::{
        VectorStore,
        lancedb::{IndexConfigDto, LanceDBVectorStore},
    },
    lancedb::index::IndexConfig,
};
use flow_like_types::anyhow;
use futures_util::{StreamExt, TryStreamExt};

#[tracing::instrument(name = "GET /apps/{app_id}/db/{table}/indices", skip(state, user))]
pub async fn get_db_indices(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Path((app_id, table)): Path<(String, String)>,
) -> Result<Json<Vec<IndexConfigDto>>, ApiError> {
    ensure_permission!(user, &app_id, &state, RolePermissions::ReadFiles);

    let credentials = state.master_credentials().await?;
    let connection = credentials.to_db(&app_id).await?.execute().await?;
    let db = LanceDBVectorStore::from_connection(connection, table).await;

    let indices = db.list_indices().await?;

    Ok(Json(indices))
}
