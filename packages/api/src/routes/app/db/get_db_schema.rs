use crate::{
    ensure_permission, error::ApiError, middleware::jwt::AppUser,
    permission::role_permission::RolePermissions, state::AppState,
};
use axum::{
    Extension, Json,
    extract::{Path, State},
};
use flow_like_storage::{
    arrow_schema::Schema,
    databases::vector::{VectorStore, lancedb::LanceDBVectorStore},
};

#[utoipa::path(
    get,
    path = "/apps/{app_id}/db/{table}/schema",
    tag = "database",
    description = "Get the table schema.",
    params(
        ("app_id" = String, Path, description = "Application ID"),
        ("table" = String, Path, description = "Table name")
    ),
    responses(
        (status = 200, description = "Table schema (Arrow JSON)", body = String, content_type = "application/json"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    security(
        ("bearer_auth" = []),
        ("api_key" = []),
        ("pat" = [])
    )
)]
#[tracing::instrument(name = "GET /apps/{app_id}/db/{table}/schema", skip(state, user))]
pub async fn get_db_schema(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Path((app_id, table)): Path<(String, String)>,
) -> Result<Json<Schema>, ApiError> {
    ensure_permission!(user, &app_id, &state, RolePermissions::ReadFiles);

    let credentials = state.master_credentials().await?;
    let connection = credentials.to_db(&app_id).await?.execute().await?;
    let db = LanceDBVectorStore::from_connection(connection, table).await;

    let schema = db.schema().await?;

    Ok(Json(schema))
}
