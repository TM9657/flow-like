use crate::{
    ensure_permission, error::ApiError, middleware::jwt::AppUser,
    permission::role_permission::RolePermissions, state::AppState,
};
use axum::{
    Extension, Json,
    extract::{Path, Query, State},
};
use flow_like::flow::board::Board;
use flow_like_types::anyhow;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct VersionQuery {
    /// expected format: "MAJOR_MINOR_PATCH", e.g. "1_0_3"
    pub version: Option<String>,
}

#[tracing::instrument(name = "GET /apps/{app_id}/templates/{template_id}", skip(state, user))]
pub async fn get_template(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Path((app_id, template_id)): Path<(String, String)>,
    Query(params): Query<VersionQuery>,
) -> Result<Json<Board>, ApiError> {
    let permission = ensure_permission!(user, &app_id, &state, RolePermissions::ReadTemplates);
    let sub = permission.sub()?;

    let version_opt = if let Some(ver_str) = params.version {
        let parts = ver_str
            .split('_')
            .map(str::parse::<u32>)
            .collect::<Result<Vec<u32>, _>>()?;
        match parts.as_slice() {
            [maj, min, pat] => Some((*maj, *min, *pat)),
            _ => {
                return Err(ApiError::internal_error(anyhow!(
                    "version must be in MAJOR_MINOR_PATCH format"
                )));
            }
        }
    } else {
        None
    };

    let template = state
        .scoped_template(
            &sub,
            &app_id,
            &template_id,
            &state,
            version_opt,
            crate::credentials::CredentialsAccess::ReadApp,
        )
        .await?;

    Ok(Json(template))
}
