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
    databases::vector::{VectorStore, lancedb::LanceDBVectorStore},
    lancedb::index::IndexConfig,
};
use flow_like_types::anyhow;
use futures_util::{StreamExt, TryStreamExt};

#[tracing::instrument(name = "GET /apps/{app_id}/db", skip(state, user))]
pub async fn list_tables(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Path(app_id): Path<String>,
) -> Result<Json<Vec<String>>, ApiError> {
    ensure_permission!(user, &app_id, &state, RolePermissions::ReadFiles);

    let credentials = state.master_credentials().await?;
    let connection = credentials.to_db(&app_id).await?.execute().await?;
    let tables = connection.table_names().execute().await?;

    Ok(Json(tables))
}
