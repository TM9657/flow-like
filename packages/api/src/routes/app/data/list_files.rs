use crate::{
    ensure_permission, error::ApiError, middleware::jwt::AppUser,
    permission::role_permission::RolePermissions, state::AppState,
};
use axum::{
    Extension, Json,
    extract::{Path, State},
};
use flow_like_storage::files::store::StorageItem;
use flow_like_types::anyhow;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ListFilesPayload {
    pub prefix: String,
}

#[tracing::instrument(name = "POST /apps/{app_id}/data/list", skip(state, user))]
pub async fn list_files(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Path(app_id): Path<String>,
    Json(payload): Json<ListFilesPayload>,
) -> Result<Json<Vec<StorageItem>>, ApiError> {
    ensure_permission!(user, &app_id, &state, RolePermissions::ReadFiles);

    let sub = user.sub()?;

    let project_dir = state
        .scoped_credentials(
            &sub,
            &app_id,
            crate::credentials::CredentialsAccess::ReadApp,
        )
        .await?;
    let project_dir = project_dir.to_store(false).await?;
    let path = project_dir
        .construct_upload(&app_id, &payload.prefix)
        .await?;

    let items = project_dir
        .as_generic()
        .list_with_delimiter(Some(&path))
        .await
        .map_err(|e| anyhow!("Failed to list items: {}", e))?;


    let dirs = items.common_prefixes.into_iter()
        .map(|p| StorageItem::from(p))
        .collect::<Vec<_>>();

    let mut items: Vec<StorageItem> = items.objects.into_iter().map(StorageItem::from).collect();
    items.extend(dirs);

    Ok(Json(items))
}
