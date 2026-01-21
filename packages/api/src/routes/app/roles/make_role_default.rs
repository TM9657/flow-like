use crate::{
    ensure_permission,
    entity::{app, role},
    error::ApiError,
    middleware::jwt::AppUser,
    permission::role_permission::RolePermissions,
    state::AppState,
};
use axum::{
    Extension, Json,
    extract::{Path, State},
};
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter};

#[tracing::instrument(
    name = "PUT /apps/{app_id}/roles/{role_id}/default",
    skip(state, user, role_id)
)]
pub async fn make_role_default(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Path((app_id, role_id)): Path<(String, String)>,
) -> Result<Json<()>, ApiError> {
    ensure_permission!(user, &app_id, &state, RolePermissions::Admin);

    let role = role::Entity::find_by_id(role_id.clone())
        .filter(role::Column::AppId.eq(app_id.clone()))
        .one(&state.db)
        .await?
        .ok_or(ApiError::NOT_FOUND)?;

    let Some(permission) = RolePermissions::from_bits(role.permissions) else {
        return Err(ApiError::FORBIDDEN);
    };

    if permission.contains(RolePermissions::Owner) {
        return Err(ApiError::FORBIDDEN);
    }

    let app = app::Entity::find_by_id(app_id.clone())
        .one(&state.db)
        .await?
        .ok_or(ApiError::NOT_FOUND)?;

    let mut app: app::ActiveModel = app.into();
    app.default_role_id = Set(Some(role_id.clone()));
    app.updated_at = Set(chrono::Utc::now().naive_utc());
    app.update(&state.db).await?;

    Ok(Json(()))
}
