use crate::{
    entity::{invitation, notification},
    error::ApiError,
    middleware::jwt::AppUser,
    state::AppState,
};
use axum::{
    Extension, Json,
    extract::{Path, Query, State},
};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect,
};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Clone, Default, Deserialize, Serialize, ToSchema)]
pub struct NotificationOverview {
    pub invites_count: u64,
    pub notifications_count: u64,
    pub unread_count: u64,
}

#[derive(Debug, Clone, Deserialize, IntoParams)]
pub struct ListNotificationsParams {
    pub limit: Option<u64>,
    pub offset: Option<u64>,
    pub unread_only: Option<bool>,
}

#[utoipa::path(
    get,
    path = "/user/notifications",
    tag = "user",
    responses(
        (status = 200, description = "Notification overview with counts", body = NotificationOverview),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
#[tracing::instrument(name = "GET /user/notifications", skip(state, user))]
pub async fn get_notifications(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
) -> Result<Json<NotificationOverview>, ApiError> {
    let sub = user.sub()?;

    let invites_count = invitation::Entity::find()
        .filter(invitation::Column::UserId.eq(sub.clone()))
        .count(&state.db)
        .await?;

    let notifications_count = notification::Entity::find()
        .filter(notification::Column::UserId.eq(sub.clone()))
        .count(&state.db)
        .await?;

    let unread_count = notification::Entity::find()
        .filter(notification::Column::UserId.eq(sub))
        .filter(notification::Column::Read.eq(false))
        .count(&state.db)
        .await?;

    Ok(Json(NotificationOverview {
        invites_count,
        notifications_count,
        unread_count,
    }))
}

#[utoipa::path(
    get,
    path = "/user/notifications/list",
    tag = "user",
    params(ListNotificationsParams),
    responses(
        (status = 200, description = "List of notifications"),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
#[tracing::instrument(name = "GET /user/notifications/list", skip(state, user))]
pub async fn list_notifications(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Query(params): Query<ListNotificationsParams>,
) -> Result<Json<Vec<notification::Model>>, ApiError> {
    let sub = user.sub()?;

    let limit = params.limit.unwrap_or(20).min(100);
    let offset = params.offset.unwrap_or(0);

    let mut query = notification::Entity::find()
        .filter(notification::Column::UserId.eq(sub))
        .order_by_desc(notification::Column::CreatedAt);

    if params.unread_only.unwrap_or(false) {
        query = query.filter(notification::Column::Read.eq(false));
    }

    let notifications = query.limit(limit).offset(offset).all(&state.db).await?;

    Ok(Json(notifications))
}

#[utoipa::path(
    post,
    path = "/user/notifications/{id}/read",
    tag = "user",
    params(
        ("id" = String, Path, description = "Notification ID")
    ),
    responses(
        (status = 200, description = "Notification marked as read"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Notification not found")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
#[tracing::instrument(name = "POST /user/notifications/{id}/read", skip(state, user))]
pub async fn mark_notification_read(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Path(notification_id): Path<String>,
) -> Result<Json<()>, ApiError> {
    let sub = user.sub()?;

    let notification = notification::Entity::find_by_id(notification_id.clone())
        .filter(notification::Column::UserId.eq(sub))
        .one(&state.db)
        .await?
        .ok_or(ApiError::NOT_FOUND)?;

    let mut active: notification::ActiveModel = notification.into();
    active.read = Set(true);
    active.read_at = Set(Some(chrono::Utc::now().naive_utc()));
    active.update(&state.db).await?;

    Ok(Json(()))
}

#[utoipa::path(
    delete,
    path = "/user/notifications/{id}",
    tag = "user",
    params(
        ("id" = String, Path, description = "Notification ID")
    ),
    responses(
        (status = 200, description = "Notification deleted"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Notification not found")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
#[tracing::instrument(name = "DELETE /user/notifications/{id}", skip(state, user))]
pub async fn delete_notification(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Path(notification_id): Path<String>,
) -> Result<Json<()>, ApiError> {
    let sub = user.sub()?;

    let notification = notification::Entity::find_by_id(notification_id.clone())
        .filter(notification::Column::UserId.eq(sub))
        .one(&state.db)
        .await?
        .ok_or(ApiError::NOT_FOUND)?;

    let active: notification::ActiveModel = notification.into();
    active.delete(&state.db).await?;

    Ok(Json(()))
}

#[utoipa::path(
    post,
    path = "/user/notifications/read-all",
    tag = "user",
    responses(
        (status = 200, description = "Number of notifications marked as read", body = u64),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
#[tracing::instrument(name = "POST /user/notifications/read-all", skip(state, user))]
pub async fn mark_all_read(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
) -> Result<Json<u64>, ApiError> {
    let sub = user.sub()?;

    let result = notification::Entity::update_many()
        .col_expr(
            notification::Column::Read,
            sea_orm::sea_query::Expr::value(true),
        )
        .col_expr(
            notification::Column::ReadAt,
            sea_orm::sea_query::Expr::value(chrono::Utc::now().naive_utc()),
        )
        .filter(notification::Column::UserId.eq(sub))
        .filter(notification::Column::Read.eq(false))
        .exec(&state.db)
        .await?;

    Ok(Json(result.rows_affected))
}
