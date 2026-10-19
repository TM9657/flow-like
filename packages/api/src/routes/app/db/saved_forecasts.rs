use axum::{
    Extension, Json,
    extract::{
        DefaultBodyLimit, FromRequest, Path, Query, Request, State, rejection::JsonRejection,
    },
    routing::{MethodRouter, get},
};
use flow_like_catalog_ml::inspection::forecasting::studio::{
    self,
    saved::{self, CreateStudioForecast, SavedStudioForecast, SavedStudioForecastSummary},
};
use serde::Deserialize;

use super::{ScopeParams, column_reader, resolve_connection, resolve_write_connection};
use crate::{
    ensure_any_permission, error::ApiError, middleware::jwt::AppUser,
    permission::role_permission::RolePermissions, state::AppState,
};

#[derive(Debug, Default, Deserialize)]
pub struct SavedForecastListParams {
    #[serde(flatten)]
    scope: ScopeParams,
    offset: Option<usize>,
    limit: Option<usize>,
}

fn validate_scope(scope: &ScopeParams) -> Result<(), ApiError> {
    if scope
        .scope
        .as_deref()
        .is_some_and(|scope| scope != "user" && scope != "project")
    {
        return Err(ApiError::bad_request("scope must be 'user' or 'project'"));
    }
    Ok(())
}

fn pagination(params: &SavedForecastListParams) -> Result<(usize, usize), ApiError> {
    let limit = params.limit.unwrap_or(100);
    if !(1..=100).contains(&limit) {
        return Err(ApiError::bad_request("limit must be between 1 and 100"));
    }
    let offset = params.offset.unwrap_or(0);
    if offset > 1000 {
        return Err(ApiError::bad_request("offset must not exceed 1000"));
    }
    Ok((offset, limit))
}

fn input_error(error: flow_like_types::Error) -> ApiError {
    if let Some(message) = studio::input_error_message(&error) {
        ApiError::bad_request(message)
    } else {
        super::table_input_error(error)
    }
}

fn body_rejection(rejection: JsonRejection) -> ApiError {
    ApiError::coded(
        rejection.status(),
        "INVALID_FORECAST_SNAPSHOT",
        rejection.body_text(),
    )
}

fn snapshot_body_limit<S: Clone + Send + Sync + 'static>(
    handler: MethodRouter<S>,
) -> MethodRouter<S> {
    handler.layer(DefaultBodyLimit::max(saved::MAX_SNAPSHOT_BYTES))
}

pub(super) fn collection_routes() -> MethodRouter<AppState> {
    snapshot_body_limit(get(list_saved_forecasts).post(create_saved_forecast))
}

#[utoipa::path(
    get,
    path = "/apps/{app_id}/db/forecasts",
    tag = "data-studio",
    description = "List saved forecast snapshots in the selected project or personal database.",
    params(
        ("app_id" = String, Path, description = "Application ID"),
        ("scope" = Option<String>, Query, description = "'user' for personal storage; omit or use 'project' for shared project storage"),
        ("offset" = Option<usize>, Query, description = "Number of snapshots to skip"),
        ("limit" = Option<usize>, Query, description = "Page size, from 1 to 100; defaults to 100")
    ),
    responses((status = 200, description = "Saved forecast summaries"), (status = 403, description = "Database or snapshot access denied")),
    security(("bearer_auth" = []), ("api_key" = []), ("pat" = []))
)]
pub async fn list_saved_forecasts(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Path(app_id): Path<String>,
    Query(params): Query<SavedForecastListParams>,
) -> Result<Json<Vec<SavedStudioForecastSummary>>, ApiError> {
    let permission = ensure_any_permission!(
        user,
        &app_id,
        &state,
        RolePermissions::ReadFiles,
        RolePermissions::ReadDatabase
    );
    validate_scope(&params.scope)?;
    let (offset, limit) = pagination(&params)?;
    let connection = resolve_connection(&state, &user, &app_id, &params.scope).await?;
    let snapshots = saved::list(
        &connection,
        &app_id,
        &column_reader(&permission, &params.scope),
        offset,
        limit,
    )
    .await
    .map_err(input_error)?;
    Ok(Json(snapshots))
}

#[utoipa::path(
    get,
    path = "/apps/{app_id}/db/forecasts/{forecast_id}",
    tag = "data-studio",
    description = "Open a saved forecast without loading its model or running inference.",
    params(
        ("app_id" = String, Path, description = "Application ID"),
        ("forecast_id" = String, Path, description = "Saved forecast ID"),
        ("scope" = Option<String>, Query, description = "'user' for personal storage; omit for project")
    ),
    responses((status = 200, description = "Saved forecast"), (status = 403, description = "Database or snapshot access denied"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []), ("api_key" = []), ("pat" = []))
)]
pub async fn get_saved_forecast(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Path((app_id, forecast_id)): Path<(String, String)>,
    Query(scope): Query<ScopeParams>,
) -> Result<Json<SavedStudioForecast>, ApiError> {
    let permission = ensure_any_permission!(
        user,
        &app_id,
        &state,
        RolePermissions::ReadFiles,
        RolePermissions::ReadDatabase
    );
    validate_scope(&scope)?;
    let connection = resolve_connection(&state, &user, &app_id, &scope).await?;
    let snapshot = saved::get(
        &connection,
        &app_id,
        &forecast_id,
        &column_reader(&permission, &scope),
    )
    .await
    .map_err(input_error)?
    .ok_or_else(|| ApiError::not_found("Saved forecast not found"))?;
    Ok(Json(snapshot))
}

#[utoipa::path(
    post,
    path = "/apps/{app_id}/db/forecasts",
    tag = "data-studio",
    description = "Save an immutable forecast snapshot. The server assigns its app, creator, ID and creation time. Project snapshots can be opened by authorized collaborators.",
    params(
        ("app_id" = String, Path, description = "Application ID"),
        ("scope" = Option<String>, Query, description = "'user' for personal storage; omit for project")
    ),
    request_body = serde_json::Value,
    responses((status = 200, description = "Created forecast snapshot"), (status = 400, description = "Invalid snapshot"), (status = 403, description = "Write or snapshot access denied"), (status = 413, description = "Snapshot exceeds 64 MiB")),
    security(("bearer_auth" = []), ("api_key" = []), ("pat" = []))
)]
pub async fn create_saved_forecast(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Path(app_id): Path<String>,
    Query(scope): Query<ScopeParams>,
    request: Request,
) -> Result<Json<SavedStudioForecast>, ApiError> {
    let permission = ensure_any_permission!(
        user,
        &app_id,
        &state,
        RolePermissions::WriteFiles,
        RolePermissions::WriteDatabase
    );
    validate_scope(&scope)?;
    let created_by = permission.identifier();
    // Check app access before buffering a potentially large completed forecast.
    let Json(payload) = Json::<CreateStudioForecast>::from_request(request, &state)
        .await
        .map_err(body_rejection)?;
    let connection = resolve_write_connection(&state, &user, &app_id, &scope).await?;
    let snapshot = saved::create(
        &connection,
        &app_id,
        &created_by,
        &column_reader(&permission, &scope),
        payload,
    )
    .await
    .map_err(input_error)?;
    Ok(Json(snapshot))
}

#[utoipa::path(
    delete,
    path = "/apps/{app_id}/db/forecasts/{forecast_id}",
    tag = "data-studio",
    description = "Delete a saved forecast from the selected database.",
    params(
        ("app_id" = String, Path, description = "Application ID"),
        ("forecast_id" = String, Path, description = "Saved forecast ID"),
        ("scope" = Option<String>, Query, description = "'user' for personal storage; omit for project")
    ),
    responses((status = 200, description = "Deleted"), (status = 403, description = "Write or snapshot access denied"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []), ("api_key" = []), ("pat" = []))
)]
pub async fn delete_saved_forecast(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Path((app_id, forecast_id)): Path<(String, String)>,
    Query(scope): Query<ScopeParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let permission = ensure_any_permission!(
        user,
        &app_id,
        &state,
        RolePermissions::WriteFiles,
        RolePermissions::WriteDatabase
    );
    validate_scope(&scope)?;
    let connection = resolve_write_connection(&state, &user, &app_id, &scope).await?;
    if !saved::delete(
        &connection,
        &app_id,
        &forecast_id,
        &column_reader(&permission, &scope),
    )
    .await
    .map_err(input_error)?
    {
        return Err(ApiError::not_found("Saved forecast not found"));
    }
    Ok(Json(serde_json::json!({"ok": true})))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Router, body::Body, http::StatusCode, routing::post};
    use tower::ServiceExt;

    #[test]
    fn scopes_and_pagination_cannot_silently_select_another_database() {
        for scope in [None, Some("user"), Some("project")] {
            assert!(
                validate_scope(&ScopeParams {
                    scope: scope.map(String::from)
                })
                .is_ok()
            );
        }
        for scope in ["", "users", "USER", "other-user", "../project"] {
            assert!(
                validate_scope(&ScopeParams {
                    scope: Some(scope.into())
                })
                .is_err()
            );
        }
        assert_eq!(
            pagination(&SavedForecastListParams::default()).unwrap(),
            (0, 100)
        );
        for limit in [0, 101] {
            assert!(
                pagination(&SavedForecastListParams {
                    limit: Some(limit),
                    ..Default::default()
                })
                .is_err()
            );
        }
        assert!(
            pagination(&SavedForecastListParams {
                offset: Some(1001),
                ..Default::default()
            })
            .is_err()
        );
    }

    #[test]
    fn snapshot_errors_keep_input_and_access_failures_distinct() {
        let invalid: flow_like_types::Error =
            studio::StudioForecastInputError("Snapshot shape is invalid".into()).into();
        let invalid = input_error(invalid);
        assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);
        assert_eq!(invalid.public_message(), Some("Snapshot shape is invalid"));
        let denied: flow_like_types::Error =
            flow_like_storage::databases::vector::schema::ColumnAccessRejected(
                "Snapshot source is restricted".into(),
            )
            .into();
        let denied = input_error(denied);
        assert_eq!(denied.status(), StatusCode::FORBIDDEN);
        let internal = input_error(flow_like_types::anyhow!("Internal storage failure"));
        assert_eq!(internal.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_ne!(internal.public_message(), Some("Internal storage failure"));
    }

    #[tokio::test]
    async fn snapshot_route_accepts_bodies_above_axums_default_limit() {
        async fn body(Json(_value): Json<serde_json::Value>) -> StatusCode {
            StatusCode::NO_CONTENT
        }
        let payload =
            serde_json::to_vec(&serde_json::json!({"snapshot": "x".repeat(3 * 1024 * 1024)}))
                .unwrap();
        let request = || {
            axum::http::Request::builder()
                .method("POST")
                .uri("/forecasts")
                .header("content-type", "application/json")
                .body(Body::from(payload.clone()))
                .unwrap()
        };
        let normal = Router::new()
            .route("/forecasts", post(body))
            .oneshot(request())
            .await
            .unwrap();
        assert_eq!(normal.status(), StatusCode::PAYLOAD_TOO_LARGE);
        let snapshots = Router::new()
            .route("/forecasts", snapshot_body_limit(post(body)))
            .layer(DefaultBodyLimit::max(2 * 1024 * 1024))
            .oneshot(request())
            .await
            .unwrap();
        assert_eq!(snapshots.status(), StatusCode::NO_CONTENT);
    }
}
