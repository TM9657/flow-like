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
    extract::{Path, State},
};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};

#[tracing::instrument(name = "GET /apps/{app_id}/routes", skip(state, user))]
pub async fn get_routes(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Path(app_id): Path<String>,
) -> Result<Json<Vec<app_route::Model>>, ApiError> {
    ensure_permission!(user, &app_id, &state, RolePermissions::ReadBoards);

    let routes = app_route::Entity::find()
        .filter(app_route::Column::AppId.eq(&app_id))
        .order_by_asc(app_route::Column::Priority)
        .all(&state.db)
        .await?;

    Ok(Json(routes))
}
