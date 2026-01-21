//! Registry index endpoints

use crate::error::ApiError;
use crate::state::AppState;
use axum::Json;
use axum::extract::{Path, State};
use flow_like_wasm::registry::{PackageVersion, RegistryEntry, RegistryIndex};

/// GET /registry/index.json
/// Returns the full registry index for offline caching
pub async fn index(State(state): State<AppState>) -> Result<Json<RegistryIndex>, ApiError> {
    let registry = state
        .wasm_registry
        .as_ref()
        .ok_or_else(|| ApiError::service_unavailable("WASM registry not configured"))?;

    let index = registry.get_index().await?;
    Ok(Json(index))
}

/// GET /registry/package/{id}
/// Returns full package entry details
pub async fn get_package(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<RegistryEntry>, ApiError> {
    let registry = state
        .wasm_registry
        .as_ref()
        .ok_or_else(|| ApiError::service_unavailable("WASM registry not configured"))?;

    let entry = registry
        .get_package(&id)
        .await?
        .ok_or_else(|| ApiError::not_found(format!("Package '{}' not found", id)))?;

    Ok(Json(entry))
}

/// GET /registry/package/{id}/versions
/// Returns all versions for a package
pub async fn get_versions(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Vec<PackageVersion>>, ApiError> {
    let registry = state
        .wasm_registry
        .as_ref()
        .ok_or_else(|| ApiError::service_unavailable("WASM registry not configured"))?;

    let entry = registry
        .get_package(&id)
        .await?
        .ok_or_else(|| ApiError::not_found(format!("Package '{}' not found", id)))?;

    Ok(Json(entry.versions))
}
