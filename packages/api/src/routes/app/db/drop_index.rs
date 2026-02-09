use crate::{
    ensure_permission, error::ApiError, middleware::jwt::AppUser,
    permission::role_permission::RolePermissions, state::AppState,
};
use axum::{
    Extension, Json,
    extract::{Path, State},
};
use flow_like_storage::databases::vector::lancedb::LanceDBVectorStore;

#[utoipa::path(
    delete,
    path = "/apps/{app_id}/db/{table}/index/{index_name}",
    tag = "database",
    description = "Remove an index from a table.",
    params(
        ("app_id" = String, Path, description = "Application ID"),
        ("table" = String, Path, description = "Table name"),
        ("index_name" = String, Path, description = "Index name")
    ),
    responses(
        (status = 200, description = "Index dropped", body = ()),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    security(
        ("bearer_auth" = []),
        ("api_key" = []),
        ("pat" = [])
    )
)]
#[tracing::instrument(
    name = "DELETE /apps/{app_id}/db/{table}/index/{index_name}",
    skip(state, user)
)]
pub async fn drop_index(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Path((app_id, table, index_name)): Path<(String, String, String)>,
) -> Result<Json<()>, ApiError> {
    ensure_permission!(user, &app_id, &state, RolePermissions::WriteFiles);

    let credentials = state.master_credentials().await?;
    let connection = credentials.to_db(&app_id).await?.execute().await?;
    let db = LanceDBVectorStore::from_connection(connection, table).await;

    db.drop_index(&index_name).await?;

    Ok(Json(()))
}
