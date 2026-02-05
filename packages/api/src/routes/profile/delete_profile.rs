use crate::{entity::profile, error::ApiError, middleware::jwt::AppUser, state::AppState};
use axum::{
    Extension, Json,
    extract::{Path, State},
};
use sea_orm::{ColumnTrait, EntityTrait, ModelTrait, QueryFilter};

#[utoipa::path(
    delete,
    path = "/profile/{profile_id}",
    tag = "profile",
    params(
        ("profile_id" = String, Path, description = "Profile ID to delete")
    ),
    responses(
        (status = 200, description = "Profile deleted successfully"),
        (status = 401, description = "Unauthorized")
    )
)]
#[tracing::instrument(name = "DELETE /profile/{profile_id}", skip(state, user))]
pub async fn delete_profile(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Path(profile_id): Path<String>,
) -> Result<Json<()>, ApiError> {
    let sub = user.sub()?;

    let profile = profile::Entity::find()
        .filter(profile::Column::UserId.eq(sub))
        .one(&state.db)
        .await?;

    if let Some(profile) = profile {
        profile.delete(&state.db).await?;
    }

    Ok(Json(()))
}
