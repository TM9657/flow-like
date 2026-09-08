use crate::{
    ensure_permission, error::ApiError, middleware::jwt::AppUser,
    permission::role_permission::RolePermissions, state::AppState,
};
use axum::{
    Extension, Json,
    extract::{Path, Query, State},
};
use flow_like::a2ui::widget::Widget;
use serde::Deserialize;
use utoipa::ToSchema;

// A deleted widget must return 404 so clients stop rendering their cached copy.
// Storage failures and unreadable payloads remain server errors.
fn map_widget_read_error(widget_id: &str, error: flow_like_types::Error) -> ApiError {
    if matches!(
        error.downcast_ref::<flow_like_storage::object_store::Error>(),
        Some(flow_like_storage::object_store::Error::NotFound { .. })
    ) {
        return ApiError::not_found(format!("Widget {widget_id} not found"));
    }
    ApiError::from(error)
}

#[derive(Deserialize, Debug, ToSchema)]
pub struct VersionQuery {
    /// expected format: "MAJOR_MINOR_PATCH", e.g. "1_0_3"
    pub version: Option<String>,
}

#[utoipa::path(
    get,
    path = "/apps/{app_id}/widgets/{widget_id}",
    tag = "widgets",
    description = "Get a widget by ID and optional version.",
    params(
        ("app_id" = String, Path, description = "Application ID"),
        ("widget_id" = String, Path, description = "Widget ID"),
        ("version" = Option<String>, Query, description = "Version in MAJOR_MINOR_PATCH format")
    ),
    responses(
        (status = 200, description = "Widget payload", body = String, content_type = "application/json"),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Widget or version not found")
    ),
    security(
        ("bearer_auth" = []),
        ("api_key" = []),
        ("pat" = [])
    )
)]
#[tracing::instrument(
    name = "GET /apps/{app_id}/widgets/{widget_id}",
    skip(state, user, params)
)]
pub async fn get_widget(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Path((app_id, widget_id)): Path<(String, String)>,
    Query(params): Query<VersionQuery>,
) -> Result<Json<Widget>, ApiError> {
    ensure_permission!(user, &app_id, &state, RolePermissions::ReadWidgets);

    let version_opt = if let Some(ver_str) = params.version {
        let parts = ver_str
            .split('_')
            .map(str::parse::<u32>)
            .collect::<Result<Vec<u32>, _>>()
            .map_err(|_| ApiError::bad_request("version must be in MAJOR_MINOR_PATCH format"))?;
        match parts.as_slice() {
            [maj, min, pat] => Some((*maj, *min, *pat)),
            _ => {
                return Err(ApiError::bad_request(
                    "version must be in MAJOR_MINOR_PATCH format",
                ));
            }
        }
    } else {
        None
    };

    let app = state.master_app(&user.sub()?, &app_id, &state).await?;

    let widget = app
        .open_widget(widget_id.clone(), version_opt)
        .await
        .map_err(|error| map_widget_read_error(&widget_id, error))?;

    Ok(Json(widget))
}

#[cfg(test)]
mod tests {
    use super::map_widget_read_error;
    use axum::http::StatusCode;
    use flow_like_storage::object_store;

    #[test]
    fn missing_widget_or_snapshot_returns_not_found() {
        for path in [
            "apps/app/widget.widget",
            "apps/app/widgets/versions/widget/1-0-0.widget",
        ] {
            let error = flow_like_types::Error::new(object_store::Error::NotFound {
                path: path.into(),
                source: Box::new(std::io::Error::from(std::io::ErrorKind::NotFound)),
            })
            .context("Failed to read widget payload");

            assert_eq!(
                map_widget_read_error("widget", error).status(),
                StatusCode::NOT_FOUND
            );
        }
    }

    #[test]
    fn transient_storage_failure_remains_a_server_error() {
        let error = flow_like_types::Error::new(object_store::Error::Generic {
            store: "test",
            source: Box::new(std::io::Error::from(std::io::ErrorKind::TimedOut)),
        });

        assert_eq!(
            map_widget_read_error("widget", error).status(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[test]
    fn corrupt_payload_remains_a_server_error() {
        let error = serde_json::from_slice::<serde_json::Value>(b"{").unwrap_err();

        assert_eq!(
            map_widget_read_error("widget", error.into()).status(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }
}
