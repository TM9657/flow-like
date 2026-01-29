use crate::{
    ensure_permission, error::ApiError, middleware::jwt::AppUser,
    permission::role_permission::RolePermissions, state::AppState,
};
use axum::{
    Extension, Json,
    extract::{Path, State},
};
use flow_like_types::anyhow;

use super::db::delete_event_from_db;

#[tracing::instrument(name = "DELETE /apps/{app_id}/events/{event_id}", skip(state, user))]
pub async fn delete_event(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Path((app_id, event_id)): Path<(String, String)>,
) -> Result<Json<()>, ApiError> {
    let permission = ensure_permission!(user, &app_id, &state, RolePermissions::WriteEvents);
    let sub = permission.sub()?;

    let mut app = state
        .scoped_app(
            &sub,
            &app_id,
            &state,
            crate::credentials::CredentialsAccess::EditApp,
        )
        .await?;

    // Delete from bucket
    app.delete_event(&event_id).await.map_err(|e| {
        tracing::error!("Failed to delete event from bucket: {}", e);
        ApiError::internal_error(anyhow!(e))
    })?;

    // Delete from database
    delete_event_from_db(&state.db, &event_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to delete event from database: {}", e);
            ApiError::internal_error(anyhow!(e))
        })?;

    Ok(Json(()))
}
