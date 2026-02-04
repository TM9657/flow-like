use crate::{
    ensure_permission, entity::technical_user, error::ApiError, middleware::jwt::AppUser,
    permission::role_permission::RolePermissions, state::AppState,
};
use axum::{
    Extension, Json,
    extract::{Path, State},
};
use sea_orm::{ColumnTrait, EntityTrait, ModelTrait, QueryFilter};

#[tracing::instrument(name = "DELETE /apps/{app_id}/api/{key_id}", skip(state, user))]
pub async fn delete_api_key(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Path((app_id, key_id)): Path<(String, String)>,
) -> Result<Json<()>, ApiError> {
    ensure_permission!(user, &app_id, &state, RolePermissions::Admin);

    let technical_user = technical_user::Entity::find_by_id(&key_id)
        .filter(technical_user::Column::AppId.eq(&app_id))
        .one(&state.db)
        .await?
        .ok_or(ApiError::NOT_FOUND)?;

    technical_user.delete(&state.db).await?;

    // Invalidate any cached auth for this key
    state.auth_cache.invalidate_all();

    Ok(Json(()))
}
