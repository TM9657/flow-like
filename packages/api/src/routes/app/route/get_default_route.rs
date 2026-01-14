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
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

#[tracing::instrument(name = "GET /apps/{app_id}/routes/default", skip(state, user))]
pub async fn get_default_route(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Path(app_id): Path<String>,
) -> Result<Json<Option<app_route::Model>>, ApiError> {
    ensure_permission!(user, &app_id, &state, RolePermissions::ExecuteEvents);

    let route = app_route::Entity::find()
        .filter(app_route::Column::AppId.eq(&app_id))
        .filter(app_route::Column::IsDefault.eq(true))
        .one(&state.db)
        .await?;

    Ok(Json(route))
}
