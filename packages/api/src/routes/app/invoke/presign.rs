use crate::{
    credentials::CredentialsAccess, ensure_permission, error::ApiError, middleware::jwt::AppUser,
    permission::role_permission::RolePermissions, state::AppState,
};
use axum::{
    Extension, Json,
    extract::{Path, State},
};
use flow_like::credentials::SharedCredentials;

#[utoipa::path(
    get,
    path = "/apps/{app_id}/invoke/presign",
    tag = "execution",
    description = "Get shared credentials for runtime execution.",
    params(
        ("app_id" = String, Path, description = "Application ID")
    ),
    responses(
        (status = 200, description = "Shared credentials", body = String, content_type = "application/json"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    security(
        ("bearer_auth" = []),
        ("api_key" = []),
        ("pat" = [])
    )
)]
#[tracing::instrument(name = "GET /apps/{app_id}/invoke/presign", skip(state, user))]
pub async fn presign(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Path(app_id): Path<String>,
) -> Result<Json<SharedCredentials>, ApiError> {
    let permission = ensure_permission!(user, &app_id, &state, RolePermissions::ExecuteEvents);

    let sub = user.sub()?;

    let mut access = CredentialsAccess::InvokeNone;

    if permission.has_permission(RolePermissions::WriteFiles) {
        access = CredentialsAccess::InvokeWrite;
    } else if permission.has_permission(RolePermissions::ReadFiles) {
        access = CredentialsAccess::InvokeRead;
    }

    let credentials = state.scoped_credentials(&sub, &app_id, access).await?;
    let credentials = credentials.into_shared_credentials();
    Ok(Json(credentials))
}
