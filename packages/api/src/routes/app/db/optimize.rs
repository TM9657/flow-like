use crate::{
    ensure_permission, error::ApiError, middleware::jwt::AppUser,
    permission::role_permission::RolePermissions, state::AppState,
};
use axum::{
    Extension, Json,
    extract::{Path, State},
};
use flow_like_storage::databases::vector::{VectorStore, lancedb::LanceDBVectorStore};
use utoipa::ToSchema;

#[derive(Debug, Clone, serde::Deserialize, ToSchema)]
pub struct OptimizePayload {
    #[serde(default)]
    pub keep_versions: bool,
}

#[utoipa::path(
    post,
    path = "/apps/{app_id}/db/{table}/optimize",
    tag = "database",
    description = "Optimize table storage and indices.",
    params(
        ("app_id" = String, Path, description = "Application ID"),
        ("table" = String, Path, description = "Table name")
    ),
    request_body = OptimizePayload,
    responses(
        (status = 200, description = "Table optimized", body = ()),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    security(
        ("bearer_auth" = []),
        ("api_key" = []),
        ("pat" = [])
    )
)]
#[tracing::instrument(name = "POST /apps/{app_id}/db/{table}/optimize", skip(state, user))]
pub async fn optimize_table(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Path((app_id, table)): Path<(String, String)>,
    Json(payload): Json<OptimizePayload>,
) -> Result<Json<()>, ApiError> {
    ensure_permission!(user, &app_id, &state, RolePermissions::WriteFiles);

    let credentials = state.master_credentials().await?;
    let connection = credentials.to_db(&app_id).await?.execute().await?;
    let db = LanceDBVectorStore::from_connection(connection, table).await;

    db.optimize(payload.keep_versions).await?;

    Ok(Json(()))
}
