use std::time::Duration;

use crate::{
    ensure_permission,
    error::ApiError,
    middleware::jwt::AppUser,
    permission::role_permission::RolePermissions,
    routes::app::data::batch::{SIGN_CONCURRENCY, validate_batch},
    routes::app::data::paths,
    state::AppState,
};
use axum::{
    Extension, Json,
    extract::{Path, State},
};
use flow_like_storage::files::store::FlowLikeStore;
use flow_like_types::{Value, create_id, json};
use futures::stream::{self, StreamExt};
use utoipa::ToSchema;

/// How long a download link is worth handing out, before the signing
/// credential's own lifetime is taken into account. See
/// [`RuntimeCredentials::signing_ttl`] — the credential, not this constant, is
/// what decides the deadline the URL actually advertises.
pub(crate) const DOWNLOAD_URL_TTL: Duration = Duration::from_secs(60 * 60 * 24);

/// Signs a GET URL per prefix, preserving request order.
///
/// A prefix that cannot be signed yields an `error` entry instead of failing
/// the batch, so one unreadable path does not cost the rest of the selection.
pub(crate) async fn sign_downloads(
    store: &FlowLikeStore,
    entries: Vec<(String, flow_like_storage::Path)>,
    ttl: Duration,
    sub: &str,
    app_id: &str,
) -> Vec<Value> {
    stream::iter(entries)
        .map(|(prefix, download_path)| async move {
            // Editor saves overwrite these paths, so a reused URL can serve old browser bytes.
            match store.sign("GET", &download_path, ttl).await {
                Ok(url) => json::json!({
                    "prefix": prefix,
                    "url": url.to_string(),
                }),
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
                    json::json!({
                        "prefix": prefix,
                        "error": format!("Failed to create signed URL, reference ID: {}", id),
                    })
                }
            }
        })
        .buffered(SIGN_CONCURRENCY)
        .collect()
        .await
}

#[derive(Debug, Clone, serde::Deserialize, ToSchema)]
pub struct DownloadFilesPayload {
    pub prefixes: Vec<String>,
}

#[utoipa::path(
    post,
    path = "/apps/{app_id}/data/download",
    tag = "data",
    description = "Create signed download URLs for file prefixes. Accepts at most 100 prefixes per request.",
    params(
        ("app_id" = String, Path, description = "Application ID")
    ),
    request_body = DownloadFilesPayload,
    responses(
        (status = 200, description = "Signed download URLs", body = String, content_type = "application/json"),
        (status = 400, description = "Bad request - no prefixes, or more than 100 in one request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    security(
        ("bearer_auth" = []),
        ("api_key" = []),
        ("pat" = [])
    )
)]
#[tracing::instrument(name = "POST /apps/{app_id}/data/download", skip(state, user, payload))]
pub async fn download_files(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Path(app_id): Path<String>,
    Json(payload): Json<DownloadFilesPayload>,
) -> Result<Json<Vec<Value>>, ApiError> {
    ensure_permission!(user, &app_id, &state, RolePermissions::ReadFiles);
    validate_batch(&payload.prefixes)?;

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
    let signing_creds = if scoped_creds.as_ref().is_azure() {
        state.master_credentials().await?
    } else {
        scoped_creds
    };
    let ttl = signing_creds.signing_ttl(DOWNLOAD_URL_TTL);
    let project_dir = signing_creds.to_store(false).await?;

    let entries = payload
        .prefixes
        .iter()
        .map(|prefix| (prefix.clone(), paths::resolve_app_upload(&app_id, prefix)))
        .collect();

    Ok(Json(
        sign_downloads(&project_dir, entries, ttl, &sub, &app_id).await,
    ))
}

#[utoipa::path(
    post,
    path = "/apps/{app_id}/data/user/download",
    tag = "data",
    description = "Create signed download URLs for your private app files. Accepts at most 100 prefixes per request.",
    params(
        ("app_id" = String, Path, description = "Application ID")
    ),
    request_body = DownloadFilesPayload,
    responses(
        (status = 200, description = "Signed download URLs", body = String, content_type = "application/json"),
        (status = 400, description = "Bad request - no prefixes, or more than 100 in one request"),
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
    name = "POST /apps/{app_id}/data/user/download",
    skip(state, user, payload)
)]
pub async fn download_user_files(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Path(app_id): Path<String>,
    Json(payload): Json<DownloadFilesPayload>,
) -> Result<Json<Vec<Value>>, ApiError> {
    ensure_permission!(user, &app_id, &state, RolePermissions::ReadFiles);
    validate_batch(&payload.prefixes)?;

    let sub = user.sub()?;

    let scoped_creds = state
        .scoped_credentials(
            &sub,
            &app_id,
            crate::credentials::CredentialsAccess::ReadUser,
        )
        .await?;

    let signing_creds = if scoped_creds.as_ref().is_azure() {
        state.master_credentials().await?
    } else {
        scoped_creds
    };
    let ttl = signing_creds.signing_ttl(DOWNLOAD_URL_TTL);
    let project_dir = signing_creds.to_store(false).await?;

    let entries = payload
        .prefixes
        .iter()
        .map(|prefix| {
            (
                prefix.clone(),
                paths::resolve_user_upload(&sub, &app_id, prefix),
            )
        })
        .collect();

    Ok(Json(
        sign_downloads(&project_dir, entries, ttl, &sub, &app_id).await,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use flow_like_storage::object_store::aws::AmazonS3Builder;
    use std::sync::Arc;

    #[tokio::test]
    async fn repeated_downloads_receive_fresh_signatures_for_mutable_files() {
        let store = FlowLikeStore::AWS(Arc::new(
            AmazonS3Builder::new()
                .with_bucket_name(format!("downloads-{}", create_id()))
                .with_region("us-east-1")
                .with_endpoint("https://storage.example.test")
                .with_access_key_id("test-access-key")
                .with_secret_access_key("test-secret-key")
                .build()
                .unwrap(),
        ));
        let entries = vec![(
            "video.mp4".to_string(),
            paths::resolve_app_upload("app-a", "video.mp4"),
        )];
        let first =
            sign_downloads(&store, entries.clone(), DOWNLOAD_URL_TTL, "user-a", "app-a").await;

        // S3 signatures encode the signing time in whole seconds.
        tokio::time::sleep(Duration::from_millis(1100)).await;

        let second = sign_downloads(&store, entries, DOWNLOAD_URL_TTL, "user-a", "app-a").await;
        assert_eq!(first[0]["prefix"], "video.mp4");
        assert_eq!(second[0]["prefix"], "video.mp4");
        let first_url = first[0]["url"].as_str().expect("first download URL");
        let second_url = second[0]["url"].as_str().expect("refreshed download URL");
        assert_ne!(first_url, second_url);
    }
}
