use crate::{
    ensure_permission, error::ApiError, middleware::jwt::AppUser,
    permission::role_permission::RolePermissions, state::AppState,
};
use axum::{
    Extension, Json,
    extract::{Path, Query, State},
};
use flow_like::flow::event::Event;
use flow_like_types::anyhow;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct VersionQuery {
    /// expected format: "MAJOR_MINOR_PATCH", e.g. "1_0_3"
    pub version: Option<String>,
}

#[tracing::instrument(name = "GET /apps/{app_id}/events/{event_id}", skip(state, user))]
pub async fn get_event(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Path((app_id, event_id)): Path<(String, String)>,
    Query(query): Query<VersionQuery>,
) -> Result<Json<Event>, ApiError> {
    let permission = ensure_permission!(user, &app_id, &state, RolePermissions::WriteEvents);
    let sub = permission.sub()?;

    let version_opt = if let Some(ver_str) = query.version {
        let parts = ver_str
            .split('_')
            .map(str::parse::<u32>)
            .collect::<Result<Vec<u32>, _>>()?;
        match parts.as_slice() {
            [maj, min, pat] => Some((*maj, *min, *pat)),
            _ => {
                return Err(ApiError::InternalError(
                    anyhow!("version must be in MAJOR_MINOR_PATCH format"),
                ));
            }
        }
    } else {
        None
    };

    let app = state.master_app(&sub, &app_id, &state).await?;
    let event = app.get_event(&event_id, version_opt).await?;

    Ok(Json(event))
}
