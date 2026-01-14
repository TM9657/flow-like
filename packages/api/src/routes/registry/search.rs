//! Package search endpoint

use crate::error::ApiError;
use crate::state::AppState;
use axum::Json;
use axum::extract::{Query, State};
use flow_like_wasm::registry::{SearchFilters, SearchResults};

/// GET /registry/search
/// Search packages with filters
pub async fn search(
    State(state): State<AppState>,
    Query(filters): Query<SearchFilters>,
) -> Result<Json<SearchResults>, ApiError> {
    let registry = state
        .wasm_registry
        .as_ref()
        .ok_or_else(|| ApiError::service_unavailable("WASM registry not configured"))?;

    let results = registry.search(&filters).await?;
    Ok(Json(results))
}
