use crate::{
    ensure_permission, error::ApiError, middleware::jwt::AppUser,
    permission::role_permission::RolePermissions, state::AppState,
};
use axum::{
    Extension, Json,
    extract::{Path, State},
};
use flow_like::flow::event::Event;

use super::db::{filter_event_secrets, get_events_for_app};

#[tracing::instrument(name = "GET /apps/{app_id}/events", skip(state, user))]
#[utoipa::path(
    get,
    path = "/apps/{app_id}/events",
    tag = "events",
    description = "List events for an app.",
    params(
        ("app_id" = String, Path, description = "Application ID")
    ),
    responses(
        (status = 200, description = "Event list", body = String, content_type = "application/json"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    security(
        ("bearer_auth" = []),
        ("api_key" = []),
        ("pat" = [])
    )
)]
pub async fn get_events(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Path(app_id): Path<String>,
) -> Result<Json<Vec<Event>>, ApiError> {
    let _permission = ensure_permission!(user, &app_id, &state, RolePermissions::WriteEvents);

    // Use database lookup instead of bucket
    let events = get_events_for_app(&state.db, &app_id).await?;

    // Filter out secret variable values from all events
    let events: Vec<Event> = events.into_iter().map(filter_event_secrets).collect();

    Ok(Json(events))
}
