//! The widget bundle of one package version, for hosts that serve widgets
//! from their own copy (desktop). It opens to whoever may load the version's
//! widgets in the web sandbox, so a project member reaches the bundle of a
//! version the project's pin covers without access to the package itself and
//! without its node binary.

use super::widget_asset::authorize_widget_version;
use super::widget_policy::invalid_app_id;
use crate::error::ApiError;
use crate::middleware::jwt::AppUser;
use crate::state::AppState;
use axum::extract::{Path, Query, State};
use axum::http::header;
use axum::response::{IntoResponse, Response};
use axum::{Extension, Json};
use flow_like_wasm_schema::widget_frame::is_valid_package_id;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Default, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct WidgetBundleQuery {
    /// Project the widget runs in. Members of a project that uses this
    /// version may load its widgets without access to the package itself.
    #[serde(default)]
    pub app_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WidgetBundleResponse {
    pub package_id: String,
    pub version: String,
    /// sha256 of the bundle file the download link serves.
    pub bundle_hash: String,
    /// Short-lived link to the bundle file.
    pub download_url: String,
}

/// GET /registry/package/{package_id}/widget-bundle/{version}
#[utoipa::path(
    get,
    path = "/registry/package/{package_id}/widget-bundle/{version}",
    tag = "registry",
    description = "Get a download link for the widgets of one package version, for apps that show widgets from their own copy. Anyone who may load the version's widgets may download them; members of a project that uses this version or a later one get them through the project. The node binary is never part of it.",
    params(
        ("package_id" = String, Path, description = "Package ID"),
        ("version" = String, Path, description = "Package version"),
        WidgetBundleQuery
    ),
    responses(
        (status = 200, description = "Link to the version's widget bundle and its hash", body = WidgetBundleResponse),
        (status = 400, description = "Malformed app id (INVALID_APP_ID)"),
        (status = 403, description = "No access to this package"),
        (status = 404, description = "Package or version not found, or the version ships no widgets"),
        (status = 503, description = "WASM registry not configured")
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_widget_bundle(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Path((package_id, version)): Path<(String, String)>,
    Query(query): Query<WidgetBundleQuery>,
) -> Result<Response, ApiError> {
    if !is_valid_package_id(&package_id) {
        return Err(ApiError::not_found(format!(
            "Package '{}' not found",
            package_id
        )));
    }
    if let Some(error) = invalid_app_id(query.app_id.as_deref()) {
        return Ok(error.into_response());
    }

    let authorized = authorize_widget_version(
        &state,
        &user,
        &package_id,
        &version,
        query.app_id.as_deref(),
    )
    .await?;
    let download_url = authorized
        .registry
        .sign_widget_bundle_url(&package_id, &version, Some(true))
        .await
        .map_err(|error| {
            tracing::warn!(
                package_id = %package_id,
                version = %version,
                %error,
                "Widget bundle URL could not be signed"
            );
            ApiError::internal(format!(
                "Could not sign the widget bundle of {package_id}@{version}"
            ))
        })?
        .ok_or_else(|| ApiError::not_found("This package version ships no widgets"))?;

    Ok((
        [(header::CACHE_CONTROL, "no-store")],
        Json(WidgetBundleResponse {
            package_id,
            version,
            bundle_hash: authorized.bundle_hash,
            download_url,
        }),
    )
        .into_response())
}
