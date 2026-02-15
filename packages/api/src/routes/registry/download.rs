//! Package download endpoint

use crate::error::ApiError;
use crate::state::AppState;
use axum::Json;
use axum::extract::State;
use flow_like_wasm::registry::{DownloadRequest, DownloadResponse};

/// POST /registry/download
/// Get download URL for a package WASM binary
/// Returns a CDN URL or signed URL for direct download
#[utoipa::path(
    post,
    path = "/registry/download",
    tag = "registry",
    request_body = DownloadRequest,
    responses(
        (status = 200, description = "Download URL and package info", body = DownloadResponse),
        (status = 404, description = "Package not found"),
        (status = 503, description = "WASM registry not configured")
    )
)]
pub async fn download(
    State(state): State<AppState>,
    Json(request): Json<DownloadRequest>,
) -> Result<Json<DownloadResponse>, ApiError> {
    let registry = state
        .wasm_registry
        .as_ref()
        .ok_or_else(|| ApiError::service_unavailable("WASM registry not configured"))?;

    // Get download URL for the package
    let (download_url, manifest, version) = registry
        .get_wasm_url(&request.package_id, request.version.as_deref())
        .await?;

    // Increment download count (fire and forget)
    let registry_clone = registry.clone();
    let package_id = request.package_id.clone();
    flow_like_types::tokio::spawn(async move {
        let _ = registry_clone.increment_downloads(&package_id).await;
    });

    Ok(Json(DownloadResponse {
        package_id: request.package_id,
        version,
        wasm_base64: String::new(), // Empty - use download_url instead
        download_url: Some(download_url),
        manifest,
    }))
}
