use crate::{
    ensure_permission,
    entity::{membership, role},
    error::ApiError,
    middleware::jwt::AppUser,
    permission::role_permission::RolePermissions,
    state::AppState,
};
use axum::{
    Extension, Json,
    extract::{Path, State},
};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, TransactionTrait};

/// Users are allowed to remove other users if they are admin. If the remove themselfes they are allowed to do so regardless of their role
#[tracing::instrument(name = "DELETE /apps/{app_id}/team/{sub}", skip(state, user))]
pub async fn remove_user(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Path((app_id, sub)): Path<(String, String)>,
) -> Result<Json<()>, ApiError> {
    let caller_sub = user.sub()?;

    if caller_sub != sub {
        ensure_permission!(user, &app_id, &state, RolePermissions::Admin);
    }

    let txn = state.db.begin().await?;

    let (membership, role) = membership::Entity::find()
        .filter(
            membership::Column::AppId
                .eq(app_id.clone())
                .and(membership::Column::UserId.eq(sub.clone())),
        )
        .find_also_related(role::Entity)
        .one(&txn)
        .await?
        .ok_or(ApiError::NOT_FOUND)?;

    if let Some(role) = role {
        let role_permissions =
            RolePermissions::from_bits(role.permissions).ok_or(ApiError::FORBIDDEN)?;

        if role_permissions.contains(RolePermissions::Owner) {
            tracing::warn!(
                "User {} is trying to remove an owner from app {}",
                sub,
                app_id
            );
            return Err(ApiError::FORBIDDEN);
        }
    }

    let membership: membership::ActiveModel = membership.into();
    membership.delete(&txn).await?;

    txn.commit().await?;

    Ok(Json(()))
}
