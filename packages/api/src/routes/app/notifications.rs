use crate::{
    ensure_permission,
    entity::{membership, notification, sea_orm_active_enums::NotificationType},
    error::ApiError,
    middleware::jwt::AppUser,
    permission::role_permission::RolePermissions,
    state::AppState,
};
use axum::{
    Extension, Json,
    extract::{Path, State},
};
use flow_like_types::create_id;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub struct CreateNotificationParams {
    /// Target user's sub. If not provided, notifies the executing user.
    pub target_user_sub: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub link: Option<String>,
    /// Run ID that triggered this notification (for tracking)
    pub run_id: Option<String>,
    /// Node ID that created this notification (for tracking)
    pub node_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateNotificationResponse {
    pub id: String,
    pub success: bool,
}

/// POST /apps/{app_id}/notifications/create
///
/// Create a notification from a workflow execution.
/// - Caller must have ExecuteBoards permission in the project
/// - If target_user_sub is provided, that user must be a member of the project
/// - If target_user_sub is not provided, notifies the executing user (from JWT sub)
/// TODO: We should send the nodeid where the notification is coming from. Otherwise we can just fire this off to annoy users
#[tracing::instrument(name = "POST /apps/{app_id}/notifications/create", skip(state, user))]
pub async fn create_notification(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Path(app_id): Path<String>,
    Json(params): Json<CreateNotificationParams>,
) -> Result<Json<CreateNotificationResponse>, ApiError> {
    // Verify caller has execution permission in the project
    let caller = ensure_permission!(user, &app_id, &state, RolePermissions::ExecuteBoards);
    let caller_sub = caller.sub()?;

    // Determine target user
    let target_sub = params.target_user_sub.unwrap_or_else(|| caller_sub.clone());

    // If targeting a different user, verify they are a member of the project
    if target_sub != caller_sub {
        let target_membership = membership::Entity::find()
            .filter(membership::Column::AppId.eq(app_id.clone()))
            .filter(membership::Column::UserId.eq(target_sub.clone()))
            .one(&state.db)
            .await?;

        if target_membership.is_none() {
            tracing::warn!(
                caller = %caller_sub,
                target = %target_sub,
                app_id = %app_id,
                "Attempted to notify user who is not a member of the project"
            );
            return Err(ApiError::Forbidden);
        }
    }

    // Create the notification
    let notification_id = create_id();
    let notification = notification::ActiveModel {
        id: Set(notification_id.clone()),
        user_id: Set(target_sub),
        app_id: Set(Some(app_id)),
        title: Set(params.title),
        description: Set(params.description),
        icon: Set(params.icon),
        link: Set(params.link),
        notification_type: Set(NotificationType::Workflow),
        read: Set(false),
        source_run_id: Set(params.run_id),
        source_node_id: Set(params.node_id),
        created_at: Set(chrono::Utc::now().naive_utc()),
        read_at: Set(None),
    };

    notification.insert(&state.db).await?;

    Ok(Json(CreateNotificationResponse {
        id: notification_id,
        success: true,
    }))
}
