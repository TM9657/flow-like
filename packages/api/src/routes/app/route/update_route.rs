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
pub struct UpdateRouteRequest {
    pub path: Option<String>,
    pub target_type: Option<RouteTargetType>,
    pub page_id: Option<String>,
    pub board_id: Option<String>,
    pub page_version: Option<String>,
    pub event_id: Option<String>,
    pub is_default: Option<bool>,
    pub priority: Option<i32>,
    pub label: Option<String>,
    pub icon: Option<String>,
}

#[tracing::instrument(name = "PUT /apps/{app_id}/routes/{route_id}", skip(state, user))]
pub async fn update_route(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Path((app_id, route_id)): Path<(String, String)>,
    Json(req): Json<UpdateRouteRequest>,
) -> Result<Json<app_route::Model>, ApiError> {
    ensure_permission!(user, &app_id, &state, RolePermissions::WriteRoutes);

    let now = Utc::now().naive_utc();

    let txn = state.db.begin().await?;

    // Find existing route
    let existing = app_route::Entity::find_by_id(&route_id)
        .filter(app_route::Column::AppId.eq(&app_id))
        .one(&txn)
        .await?
        .ok_or(ApiError::NOT_FOUND)?;

    // If setting this route as default, unset any existing default
    if let Some(true) = req.is_default {
        let existing_default = app_route::Entity::find()
            .filter(app_route::Column::AppId.eq(&app_id))
            .filter(app_route::Column::IsDefault.eq(true))
            .filter(app_route::Column::Id.ne(&route_id))
            .one(&txn)
            .await?;

        if let Some(default_route) = existing_default {
            let mut active: app_route::ActiveModel = default_route.into();
            active.is_default = Set(false);
            active.updated_at = Set(now);
            active.update(&txn).await?;
        }
    }

    let mut active_model: app_route::ActiveModel = existing.into();
    active_model.updated_at = Set(now);

    if let Some(path) = req.path {
        active_model.path = Set(path);
    }
    if let Some(target_type) = req.target_type {
        active_model.target_type = Set(target_type);
    }
    if req.page_id.is_some() {
        active_model.page_id = Set(req.page_id);
    }
    if req.board_id.is_some() {
        active_model.board_id = Set(req.board_id);
    }
    if req.page_version.is_some() {
        active_model.page_version = Set(req.page_version);
    }
    if req.event_id.is_some() {
        active_model.event_id = Set(req.event_id);
    }
    if let Some(is_default) = req.is_default {
        active_model.is_default = Set(is_default);
    }
    if let Some(priority) = req.priority {
        active_model.priority = Set(priority);
    }
    if req.label.is_some() {
        active_model.label = Set(req.label);
    }
    if req.icon.is_some() {
        active_model.icon = Set(req.icon);
    }

    let route = active_model.update(&txn).await?;
    txn.commit().await?;

    Ok(Json(route))
}
