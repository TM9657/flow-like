//! Submit review for a package

use crate::error::ApiError;
use crate::middleware::jwt::AppUser;
use crate::permission::global_permission::GlobalPermission;
use crate::routes::registry::server::{PackageReview, ReviewRequest};
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::{Extension, Json};

#[utoipa::path(
    post,
    path = "/admin/packages/{package_id}/review",
    tag = "admin",
    params(
        ("package_id" = String, Path, description = "Package ID to review")
    ),
    request_body = ReviewRequest,
    responses(
        (status = 200, description = "Review submitted", body = PackageReview),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    )
)]
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
