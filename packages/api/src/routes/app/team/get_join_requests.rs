use crate::{
    ensure_permission, entity::join_queue, error::ApiError, middleware::jwt::AppUser,
    permission::role_permission::RolePermissions, routes::LanguageParams, state::AppState,
};
use axum::{
    Extension, Json,
    extract::{Path, Query, State},
};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};

#[tracing::instrument(name = "GET /apps/{app_id}/team/queue", skip(state, user))]
pub async fn get_join_requests(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Path(app_id): Path<String>,
    Query(params): Query<LanguageParams>,
) -> Result<Json<Vec<join_queue::Model>>, ApiError> {
    ensure_permission!(user, &app_id, &state, RolePermissions::Admin);

    let links = join_queue::Entity::find()
        .order_by_asc(join_queue::Column::CreatedAt)
        .filter(join_queue::Column::AppId.eq(app_id.clone()))
        .limit(params.limit)
        .offset(params.offset)
        .all(&state.db)
        .await?;

    Ok(Json(links))
}
