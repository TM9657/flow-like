use crate::{
    ensure_permission, entity::invite_link, error::ApiError, middleware::jwt::AppUser,
    permission::role_permission::RolePermissions, state::AppState,
};
use axum::{
    Extension, Json,
    extract::{Path, State},
};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

#[utoipa::path(
    get,
    path = "/apps/{app_id}/team/link",
    tag = "team",
    description = "List invite links for the app.",
    params(
        ("app_id" = String, Path, description = "Application ID")
    ),
    responses(
        (status = 200, description = "Invite links", body = String, content_type = "application/json"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    security(
        ("bearer_auth" = []),
        ("api_key" = []),
        ("pat" = [])
    )
)]
#[tracing::instrument(name = "GET /apps/{app_id}/team/link", skip(state, user))]
pub async fn get_invite_links(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Path(app_id): Path<String>,
) -> Result<Json<Vec<invite_link::Model>>, ApiError> {
    ensure_permission!(user, &app_id, &state, RolePermissions::Admin);

    let links = invite_link::Entity::find()
        .filter(invite_link::Column::AppId.eq(app_id.clone()))
        .all(&state.db)
        .await?;

    Ok(Json(links))
}
