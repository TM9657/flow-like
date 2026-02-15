//! Package search endpoint

use crate::error::ApiError;
use crate::state::AppState;
use axum::Json;
use axum::extract::{Query, State};
use flow_like_wasm::registry::{SearchFilters, SearchResults};

/// GET /registry/search
/// Search packages with filters
#[utoipa::path(
    get,
    path = "/registry/search",
    tag = "registry",
    params(
        ("query" = Option<String>, Query, description = "Search query matching name, description, keywords"),
        ("category" = Option<String>, Query, description = "Filter by category"),
        ("keywords" = Option<Vec<String>>, Query, description = "Filter by keywords"),
        ("author" = Option<String>, Query, description = "Filter by author"),
        ("verified_only" = Option<bool>, Query, description = "Only show verified packages"),
        ("include_deprecated" = Option<bool>, Query, description = "Include deprecated packages"),
        ("offset" = Option<usize>, Query, description = "Pagination offset"),
        ("limit" = Option<usize>, Query, description = "Pagination limit"),
        ("sort_by" = Option<String>, Query, description = "Sort field: relevance, name, downloads, updated_at, created_at"),
        ("sort_desc" = Option<bool>, Query, description = "Sort direction (descending if true)")
    ),
    responses(
        (status = 200, description = "Search results", body = SearchResults),
        (status = 503, description = "WASM registry not configured")
    )
)]
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
