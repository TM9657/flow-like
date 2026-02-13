//! Get registry statistics

use crate::error::ApiError;
use crate::middleware::jwt::AppUser;
use crate::permission::global_permission::GlobalPermission;
use crate::state::AppState;
use axum::extract::State;
use axum::{Extension, Json};
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
pub struct RegistryStatsResponse {
    pub total_packages: i64,
    pub total_versions: i64,
    pub total_downloads: i64,
    pub pending_review: i64,
    pub active_packages: i64,
    pub rejected_packages: i64,
}

#[utoipa::path(
    get,
    path = "/admin/packages/stats",
    tag = "admin",
    responses(
        (status = 200, description = "Registry statistics", body = RegistryStatsResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    )
)]
pub async fn get_stats(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
) -> Result<Json<RegistryStatsResponse>, ApiError> {
    user.check_global_permission(&state, GlobalPermission::ManagePackages)
        .await?;

    let registry = state
        .wasm_registry
        .as_ref()
        .ok_or_else(|| ApiError::bad_request("WASM registry not configured"))?;

    let stats = registry.get_stats().await?;

    Ok(Json(RegistryStatsResponse {
        total_packages: stats.total_packages,
        total_versions: stats.total_versions,
        total_downloads: stats.total_downloads,
        pending_review: stats.pending_review,
        active_packages: stats.active_packages,
        rejected_packages: stats.rejected_packages,
    }))
}
