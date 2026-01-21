use crate::{
    ensure_permission, entity::page, error::ApiError, middleware::jwt::AppUser,
    permission::role_permission::RolePermissions, state::AppState,
};
use axum::{
    Extension, Json,
    extract::{Path, State},
};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

#[tracing::instrument(name = "DELETE /apps/{app_id}/pages/{page_id}", skip(state, user))]
pub async fn delete_page(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Path((app_id, page_id)): Path<(String, String)>,
) -> Result<Json<()>, ApiError> {
    ensure_permission!(user, &app_id, &state, RolePermissions::WriteBoards);

    let mut app = state
        .scoped_app(
            &user.sub()?,
            &app_id,
            &state,
            crate::credentials::CredentialsAccess::EditApp,
        )
        .await?;

    // Delete from bucket storage
    app.delete_page(&page_id).await?;

    app.page_ids.retain(|id| id != &page_id);
    app.save().await?;

    // Delete from DB
    page::Entity::delete_many()
        .filter(
            page::Column::AppId
                .eq(app_id.clone())
                .and(page::Column::Id.eq(page_id.clone())),
        )
        .exec(&state.db)
        .await?;

    Ok(Json(()))
}
