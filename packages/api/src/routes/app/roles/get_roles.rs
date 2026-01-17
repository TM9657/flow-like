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
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, TransactionTrait};

#[tracing::instrument(name = "GET /apps/{app_id}/roles", skip(state, user))]
pub async fn get_roles(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Path(app_id): Path<String>,
) -> Result<Json<(Option<String>, Vec<role::Model>)>, ApiError> {
    ensure_permission!(user, &app_id, &state, RolePermissions::ReadRoles);

    let txn = state.db.begin().await?;

    let app = app::Entity::find_by_id(app_id.clone())
        .one(&txn)
        .await?
        .ok_or_else(|| {
            tracing::warn!("App {} not found", app_id);
            ApiError::NOT_FOUND
        })?;

    let roles = role::Entity::find()
        .filter(role::Column::AppId.eq(app_id.clone()))
        .all(&txn)
        .await?;

    Ok(Json((app.default_role_id, roles)))
}
