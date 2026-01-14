//! List packages for admin

use crate::error::ApiError;
use crate::middleware::jwt::AppUser;
use crate::permission::global_permission::GlobalPermission;
use crate::routes::registry::server::PackageDetails;
use crate::state::AppState;
use axum::extract::{Query, State};
use axum::{Extension, Json};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub offset: Option<usize>,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct ListResponse {
    pub packages: Vec<PackageDetails>,
    pub total_count: usize,
    pub offset: usize,
    pub limit: usize,
}

/// GET /admin/packages
pub async fn get_packages(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Query(query): Query<ListQuery>,
) -> Result<Json<ListResponse>, ApiError> {
    user.check_global_permission(&state, GlobalPermission::ManagePackages)
        .await?;

    let registry = state
        .wasm_registry
        .as_ref()
        .ok_or_else(|| ApiError::bad_request("WASM registry not configured"))?;

    let offset = query.offset.unwrap_or(0);
    let limit = query.limit.unwrap_or(20).min(100);

    let (packages, total_count) = registry
        .list_packages_admin(query.status.as_deref(), offset, limit)
        .await?;

    Ok(Json(ListResponse {
        packages,
        total_count,
        offset,
        limit,
    }))
}
