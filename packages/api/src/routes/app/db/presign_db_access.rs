//! Presign LanceDB access endpoint
//!
//! This endpoint provides presigned access to LanceDB tables, allowing clients to directly
//! query the database without proxying through the API. This is useful for performance-sensitive
//! operations and reducing server load.
//!
//! The endpoint supports both read-only and read-write access based on user permissions.

use crate::{
    credentials::{CredentialsAccess, RuntimeCredentials},
    ensure_permission,
    error::ApiError,
    middleware::jwt::AppUser,
    permission::role_permission::RolePermissions,
    state::AppState,
};
use axum::{
    Extension, Json,
    extract::{Path, State},
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct PresignDbAccessRequest {
    /// Name of the LanceDB table to access
    pub table_name: String,
    /// Access mode: "read" or "write"
    #[serde(default = "default_access_mode")]
    pub access_mode: String,
}

fn default_access_mode() -> String {
    "read".to_string()
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PresignDbAccessResponse {
    /// Shared credentials for direct storage access
    pub shared_credentials: serde_json::Value,
    /// Base database path for this app (apps/{app_id}/storage/db)
    pub db_path: String,
    /// The table name
    pub table_name: String,
    /// Access mode granted
    pub access_mode: String,
    /// Expiration time (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiration: Option<chrono::DateTime<chrono::Utc>>,
}

/// Presign access to a LanceDB table
///
/// This endpoint generates presigned credentials for direct client-side access to LanceDB.
/// The credentials are scoped to the specific app and user, with permissions based on the
/// requested access mode and user's role permissions.
///
/// # Access Modes
/// - `read`: Read-only access (requires ReadFiles permission)
/// - `write`: Read-write access (requires WriteFiles permission)
///
/// # Security
/// - Credentials are temporary and scoped to the specific app
/// - Access is restricted based on user permissions
/// - Different storage providers (S3, Azure, GCP, R2) use appropriate presigning mechanisms
///
/// # Example Response (AWS/R2)
/// ```json
/// {
///   "provider": "aws",
///   "uri": "s3://bucket/apps/app-id/storage/db",
///   "storage_options": {
///     "aws_access_key_id": "ASIA...",
///     "aws_secret_access_key": "...",
///     "aws_session_token": "...",
///     "aws_region": "us-east-1"
///   },
///   "table_name": "my_table",
///   "access_mode": "read",
///   "expiration": "2026-02-06T12:00:00Z"
/// }
/// ```
///
/// # Example Response (Azure)
/// ```json
/// {
///   "provider": "azure",
///   "uri": "az://container/apps/app-id/storage/db",
///   "storage_options": {
///     "azure_storage_account_name": "account",
///     "azure_storage_sas_token": "..."
///   },
///   "table_name": "my_table",
///   "access_mode": "read",
///   "expiration": "2026-02-06T12:00:00Z"
/// }
/// ```
#[utoipa::path(
    post,
    path = "/apps/{app_id}/db/presign",
    tag = "database",
    description = "Get shared credentials for direct LanceDB access.",
    params(
        ("app_id" = String, Path, description = "Application ID")
    ),
    request_body = PresignDbAccessRequest,
    responses(
        (status = 200, description = "Presigned database access credentials", body = PresignDbAccessResponse),
        (status = 400, description = "Bad request - invalid access mode"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - insufficient permissions"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = []),
        ("api_key" = []),
        ("pat" = [])
    )
)]
#[tracing::instrument(
    name = "POST /apps/{app_id}/db/presign",
    skip(state, user),
    fields(app_id = %app_id, table = %payload.table_name, mode = %payload.access_mode)
)]
pub async fn presign_db_access(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Path(app_id): Path<String>,
    Json(payload): Json<PresignDbAccessRequest>,
) -> Result<Json<PresignDbAccessResponse>, ApiError> {
    // Validate access mode
    let access_mode = payload.access_mode.to_lowercase();
    if access_mode != "read" && access_mode != "write" {
        return Err(ApiError::bad_request(
            "access_mode must be either 'read' or 'write'".to_string(),
        ));
    }

    // Check permissions based on requested access mode
    let (required_permission, credentials_access) = if access_mode == "write" {
        (RolePermissions::WriteFiles, CredentialsAccess::InvokeWrite)
    } else {
        (RolePermissions::ReadFiles, CredentialsAccess::InvokeRead)
    };

    let permission = ensure_permission!(user, &app_id, &state, required_permission);
    let sub = permission.sub()?;

    // Get scoped credentials for the user
    let scoped_credentials = RuntimeCredentials::scoped(&sub, &app_id, &state, credentials_access)
        .await
        .map_err(|e| {
            tracing::error!("Failed to generate scoped credentials: {}", e);
            ApiError::internal("Failed to generate database access credentials")
        })?;

    let shared_credentials = serde_json::to_value(
        scoped_credentials.clone().into_shared_credentials(),
    )
    .map_err(|e| {
        tracing::error!("Failed to serialize shared credentials: {}", e);
        ApiError::internal("Failed to serialize shared credentials")
    })?;

    let db_path = format!("apps/{}/storage/db", app_id);

    // Get expiration time if available
    let expiration = get_credentials_expiration(&scoped_credentials);

    Ok(Json(PresignDbAccessResponse {
        shared_credentials,
        db_path,
        table_name: payload.table_name,
        access_mode,
        expiration,
    }))
}

/// Get expiration time from credentials if available
fn get_credentials_expiration(
    credentials: &RuntimeCredentials,
) -> Option<chrono::DateTime<chrono::Utc>> {
    match credentials {
        #[cfg(feature = "aws")]
        RuntimeCredentials::Aws(aws) => aws.expiration,
        #[cfg(feature = "azure")]
        RuntimeCredentials::Azure(azure) => azure.expiration,
        #[cfg(feature = "gcp")]
        RuntimeCredentials::Gcp(gcp) => gcp.expiration,
        #[cfg(feature = "r2")]
        RuntimeCredentials::R2(r2) => r2.expiration,
    }
}
