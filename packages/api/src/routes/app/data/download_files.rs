use std::time::Duration;

use crate::{
    ensure_permission, error::ApiError, middleware::jwt::AppUser,
    permission::role_permission::RolePermissions, state::AppState,
};
use axum::{
    Extension, Json,
    extract::{Path, State},
};
use flow_like_types::{Value, create_id, json};

const MAX_PREFIXES: usize = 100;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct DownloadFilesPayload {
    pub prefixes: Vec<String>,
}

#[tracing::instrument(name = "POST /apps/{app_id}/data/download", skip(state, user))]
pub async fn download_files(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Path(app_id): Path<String>,
    Json(payload): Json<DownloadFilesPayload>,
) -> Result<Json<Vec<Value>>, ApiError> {
    ensure_permission!(user, &app_id, &state, RolePermissions::ReadFiles);

    let sub = user.sub()?;

    // Get scoped credentials first to check the provider type
    let scoped_creds = state
        .scoped_credentials(
            &sub,
            &app_id,
            crate::credentials::CredentialsAccess::ReadApp,
        )
        .await?;

    // Azure SAS tokens cannot generate new signed URLs, so use master credentials for Azure
    let project_dir = if scoped_creds.as_ref().is_azure() {
        state.master_credentials().await?.to_store(false).await?
    } else {
        scoped_creds.to_store(false).await?
    };

    let mut urls = Vec::with_capacity(payload.prefixes.len());

    for prefix in payload.prefixes.iter().take(MAX_PREFIXES) {
        let upload_dir = flow_like_storage::Path::from(String::from(prefix));
        let signed_url = match project_dir
            .sign("GET", &upload_dir, Duration::from_secs(60 * 60 * 24))
            .await
        {
            Ok(url) => url,
            Err(e) => {
                let id = create_id();
                tracing::error!(
                    "[{}] Failed to sign URL for prefix '{}': {:?} [sent by {} for project {}]",
                    id,
                    prefix,
                    e,
                    sub,
                    app_id
                );
                urls.push(json::json!({
                    "prefix": prefix,
                    "error": format!("Failed to create signed URL, reference ID: {}", id),
                }));
                continue;
            }
        };

        urls.push(json::json!({
            "prefix": prefix,
            "url": signed_url.to_string(),
        }));
    }

    Ok(Json(urls))
}
