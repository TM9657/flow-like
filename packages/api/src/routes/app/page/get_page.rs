use crate::{
    ensure_permission, error::ApiError, middleware::jwt::AppUser,
    permission::role_permission::RolePermissions, state::AppState,
};
use axum::{
    Extension, Json,
    extract::{Path, Query, State},
};
use flow_like::a2ui::widget::Page;
use flow_like_types::anyhow;
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};

#[derive(Deserialize, Debug, IntoParams, ToSchema)]
pub struct VersionQuery {
    /// expected format: "MAJOR_MINOR_PATCH", e.g. "1_0_3"
    pub version: Option<String>,
}

#[utoipa::path(
    get,
    path = "/apps/{app_id}/pages/{page_id}",
    tag = "pages",
    params(
        ("app_id" = String, Path, description = "Application ID"),
        ("page_id" = String, Path, description = "Page ID"),
        VersionQuery
    ),
    responses(
        (status = 200, description = "Page details", body = Object),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Page not found")
    )
)]
#[tracing::instrument(name = "GET /apps/{app_id}/pages/{page_id}", skip(state, user))]
pub async fn get_page(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Path((app_id, page_id)): Path<(String, String)>,
    Query(params): Query<VersionQuery>,
) -> Result<Json<Page>, ApiError> {
    ensure_permission!(user, &app_id, &state, RolePermissions::ExecuteEvents);

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

    let app = state.master_app(&user.sub()?, &app_id, &state).await?;

    let page = app.open_page(page_id, version_opt).await?;

    Ok(Json(page))
}
