use crate::{
    ensure_permission,
    entity::{app_route, sea_orm_active_enums::RouteTargetType},
    error::ApiError,
    middleware::jwt::AppUser,
    permission::role_permission::RolePermissions,
    state::AppState,
};
use axum::{
    Extension, Json,
    extract::{Path, State},
};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set, TransactionTrait};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateRouteRequest {
    pub id: String,
    pub path: String,
    pub target_type: RouteTargetType,
    pub page_id: Option<String>,
    pub board_id: Option<String>,
    pub page_version: Option<String>,
    pub event_id: Option<String>,
    pub is_default: Option<bool>,
    pub priority: Option<i32>,
    pub label: Option<String>,
    pub icon: Option<String>,
}

#[tracing::instrument(name = "POST /apps/{app_id}/routes", skip(state, user))]
pub async fn create_route(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Path(app_id): Path<String>,
    Json(req): Json<CreateRouteRequest>,
) -> Result<Json<app_route::Model>, ApiError> {
    ensure_permission!(user, &app_id, &state, RolePermissions::WriteRoutes);

    let now = Utc::now().naive_utc();
    let is_default = req.is_default.unwrap_or(false);

    let txn = state.db.begin().await?;

    // If this route is set as default, unset any existing default
    if is_default {
        let existing_default = app_route::Entity::find()
            .filter(app_route::Column::AppId.eq(&app_id))
            .filter(app_route::Column::IsDefault.eq(true))
            .one(&txn)
            .await?;

        if let Some(existing) = existing_default {
            let mut active: app_route::ActiveModel = existing.into();
            active.is_default = Set(false);
            active.updated_at = Set(now);
            active.update(&txn).await?;
        }
    }

    let active_model = app_route::ActiveModel {
        id: Set(req.id),
        app_id: Set(app_id),
        path: Set(req.path),
        target_type: Set(req.target_type),
        page_id: Set(req.page_id),
        board_id: Set(req.board_id),
        page_version: Set(req.page_version),
        event_id: Set(req.event_id),
        is_default: Set(is_default),
        priority: Set(req.priority.unwrap_or(0)),
        label: Set(req.label),
        icon: Set(req.icon),
        created_at: Set(now),
        updated_at: Set(now),
    };

    let route = active_model.insert(&txn).await?;
    txn.commit().await?;

    Ok(Json(route))
}
