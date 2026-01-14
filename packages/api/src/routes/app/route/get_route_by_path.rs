use crate::{
    ensure_permission,
    entity::app_route,
    error::ApiError,
    middleware::jwt::AppUser,
    permission::role_permission::RolePermissions,
    state::AppState,
};
use axum::{
    Extension, Json,
    extract::{Path, Query, State},
};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct RoutePathQuery {
    pub path: String,
}

#[tracing::instrument(name = "GET /apps/{app_id}/routes/by-path", skip(state, user))]
pub async fn get_route_by_path(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Path(app_id): Path<String>,
    Query(query): Query<RoutePathQuery>,
) -> Result<Json<Option<app_route::Model>>, ApiError> {
    ensure_permission!(user, &app_id, &state, RolePermissions::ReadBoards);

    let route = app_route::Entity::find()
        .filter(app_route::Column::AppId.eq(&app_id))
        .filter(app_route::Column::Path.eq(&query.path))
        .one(&state.db)
        .await?;

    Ok(Json(route))
}
