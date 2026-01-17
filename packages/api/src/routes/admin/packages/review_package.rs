//! Submit review for a package

use crate::error::ApiError;
use crate::middleware::jwt::AppUser;
use crate::permission::global_permission::GlobalPermission;
use crate::routes::registry::server::{PackageReview, ReviewRequest};
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::{Extension, Json};

/// POST /admin/packages/{package_id}/review
pub async fn review_package(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Path(package_id): Path<String>,
    Json(review): Json<ReviewRequest>,
) -> Result<Json<PackageReview>, ApiError> {
    user.check_global_permission(&state, GlobalPermission::ManagePackages)
        .await?;

    let sub = user.sub()?;

    let registry = state
        .wasm_registry
        .as_ref()
        .ok_or_else(|| ApiError::bad_request("WASM registry not configured"))?;

    let result = registry.submit_review(&package_id, &sub, review).await?;

    Ok(Json(result))
}
