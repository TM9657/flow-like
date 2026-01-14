//! Update package status

use crate::error::ApiError;
use crate::middleware::jwt::AppUser;
use crate::permission::global_permission::GlobalPermission;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::{Extension, Json};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct UpdateRequest {
    pub status: Option<String>,
    pub verified: Option<bool>,
}

/// PATCH /admin/packages/{package_id}
pub async fn update_package(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Path(package_id): Path<String>,
    Json(request): Json<UpdateRequest>,
) -> Result<Json<()>, ApiError> {
    user.check_global_permission(&state, GlobalPermission::ManagePackages)
        .await?;

    let registry = state
        .wasm_registry
        .as_ref()
        .ok_or_else(|| ApiError::bad_request("WASM registry not configured"))?;

    if let Some(status) = &request.status {
        registry
            .update_status(&package_id, status, request.verified)
            .await?;
    } else if let Some(verified) = request.verified {
        registry
            .update_status(&package_id, "active", Some(verified))
            .await?;
    }

    Ok(Json(()))
}
