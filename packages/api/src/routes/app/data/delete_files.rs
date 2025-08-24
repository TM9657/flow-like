use crate::{
    ensure_permission, error::ApiError, middleware::jwt::AppUser,
    permission::role_permission::RolePermissions, state::AppState,
};
use axum::{
    Extension, Json,
    extract::{Path, State},
};
use flow_like_types::anyhow;
use futures_util::{StreamExt, TryStreamExt};

#[derive(Debug, Clone, serde::Deserialize)]
pub struct DeleteFilesPayload {
    pub prefixes: Vec<String>,
}

#[tracing::instrument(name = "DELETE /apps/{app_id}/data", skip(state, user))]
pub async fn delete_files(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Path(app_id): Path<String>,
    Json(payload): Json<DeleteFilesPayload>,
) -> Result<Json<()>, ApiError> {
    ensure_permission!(user, &app_id, &state, RolePermissions::WriteFiles);
    let sub = user.sub()?;

    let project_dir = state
        .scoped_credentials(
            &sub,
            &app_id,
            crate::credentials::CredentialsAccess::EditApp,
        )
        .await?;
    let project_dir = project_dir.to_store(false).await?;
    let generic = project_dir.as_generic();

    for prefix in payload.prefixes.iter() {
        let upload_dir = project_dir.construct_upload(&app_id, prefix).await?;
        let locations = generic
            .list(Some(&upload_dir))
            .map_ok(|m| m.location)
            .boxed();
        generic
            .delete_stream(locations)
            .try_collect::<Vec<flow_like_storage::Path>>()
            .await
            .map_err(|e| anyhow!("Failed to delete stream: {}", e))?;
        generic
            .delete(&upload_dir)
            .await
            .map_err(|e| anyhow!("Failed to delete path: {}", e))?;
    }

    Ok(Json(()))
}
