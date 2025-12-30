//! Public key endpoint for backend JWT verification
//!
//! Exposes the JWKS (JSON Web Key Set) for all clients to verify JWTs.
//! This allows executors, realtime clients, and other services to verify
//! JWTs without needing direct access to the public key.

use crate::backend_jwt::{self, BackendJwtError, Jwks};
use crate::error::ApiError;
use axum::Json;

/// GET /execution/.well-known/jwks.json
///
/// Returns the JWKS for backend JWT verification.
/// All clients can fetch this to verify JWTs without needing the public key env var.
#[tracing::instrument(name = "GET /execution/.well-known/jwks.json")]
pub async fn get_execution_jwks() -> Result<Json<Jwks>, ApiError> {
    let jwks = backend_jwt::get_jwks().map_err(|e: BackendJwtError| {
        tracing::error!(error = %e, "Failed to get backend JWKS");
        ApiError::InternalError(e.into())
    })?;

    Ok(Json(jwks))
}
