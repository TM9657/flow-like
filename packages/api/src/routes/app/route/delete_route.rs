use crate::{
    ensure_permission,
    entity::app_route,
    error::ApiError,
    middleware::jwt::AppUser,
    permission::role_permission::RolePermissions,
    state::AppState,
};
use axum::{
    Extension,
    extract::{Path, State},
};
use sea_orm::{ColumnTrait, EntityTrait, ModelTrait, QueryFilter};

#[tracing::instrument(name = "DELETE /apps/{app_id}/routes/{route_id}", skip(state, user))]
pub async fn delete_route(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Path((app_id, route_id)): Path<(String, String)>,
) -> Result<(), ApiError> {
    ensure_permission!(user, &app_id, &state, RolePermissions::WriteRoutes);

    let route = app_route::Entity::find_by_id(&route_id)
        .filter(app_route::Column::AppId.eq(&app_id))
        .one(&state.db)
        .await?
        .ok_or(ApiError::NotFound)?;

    route.delete(&state.db).await?;

    Ok(())
}
