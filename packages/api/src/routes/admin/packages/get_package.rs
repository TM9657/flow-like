//! Get single package details for admin

use crate::error::ApiError;
use crate::middleware::jwt::AppUser;
use crate::permission::global_permission::GlobalPermission;
use crate::routes::registry::server::{PackageDetails, PackageReview};
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::{Extension, Json};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct PackageDetailResponse {
    pub package: PackageDetails,
    pub reviews: Vec<PackageReview>,
}

/// GET /admin/packages/{package_id}
pub async fn get_package(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Path(package_id): Path<String>,
) -> Result<Json<PackageDetailResponse>, ApiError> {
    user.check_global_permission(&state, GlobalPermission::ManagePackages)
        .await?;

    let registry = state
        .wasm_registry
        .as_ref()
        .ok_or_else(|| ApiError::bad_request("WASM registry not configured"))?;

    let package = registry
        .get_package_admin(&package_id)
        .await?
        .ok_or(ApiError::NotFound)?;

    let reviews = registry.get_reviews(&package_id).await?;

    Ok(Json(PackageDetailResponse { package, reviews }))
}
