//! Package publish endpoint

use crate::error::ApiError;
use crate::middleware::jwt::AppUser;
use crate::state::AppState;
use axum::{Extension, Json};
use axum::extract::State;
use flow_like_wasm::registry::{PublishRequest, PublishResponse};

/// POST /registry/publish
/// Publish a new package or version (requires admin approval)
pub async fn publish(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Json(request): Json<PublishRequest>,
) -> Result<Json<PublishResponse>, ApiError> {
    // Require authentication for publishing
    let sub = user.sub().map_err(|_| {
        ApiError::unauthorized("Authentication required for publishing")
    })?;

    if sub.is_empty() {
        return Err(ApiError::unauthorized(
            "Authentication required for publishing",
        ));
    }

    let registry = state
        .wasm_registry
        .as_ref()
        .ok_or_else(|| ApiError::service_unavailable("WASM registry not configured"))?;

    // Validate manifest
    if let Err(errors) = request.manifest.validate() {
        return Err(ApiError::bad_request(format!(
            "Invalid manifest: {}",
            errors.join(", ")
        )));
    }

    // Decode WASM
    use base64::Engine;
    let wasm_data = base64::engine::general_purpose::STANDARD
        .decode(&request.wasm_base64)
        .map_err(|e| ApiError::bad_request(format!("Invalid WASM base64: {}", e)))?;

    // Validate WASM magic bytes
    if wasm_data.len() < 8
        || &wasm_data[0..4] != b"\0asm"
    {
        return Err(ApiError::bad_request("Invalid WASM binary"));
    }

    // Try to get user email from the database user record
    let email = match state.db.clone() {
        db => {
            use crate::entity::user;
            use sea_orm::EntityTrait;
            user::Entity::find_by_id(&sub)
                .one(&db)
                .await
                .ok()
                .flatten()
                .and_then(|u| u.email)
        }
    };

    // Publish with submitter info
    let response = registry
        .publish(request.manifest.clone(), wasm_data, Some(sub), email)
        .await?;

    Ok(Json(response))
}
