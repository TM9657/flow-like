use crate::{
    ensure_permission,
    entity::{role, technical_user},
    error::ApiError,
    middleware::jwt::AppUser,
    permission::role_permission::RolePermissions,
    state::AppState,
};
use axum::{
    Extension, Json,
    extract::{Path, State},
};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct ApiKeyInfo {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub role_id: Option<String>,
    pub role_name: Option<String>,
    pub role_permissions: Option<i64>,
    pub valid_until: Option<i64>,
    pub created_at: i64,
}

#[tracing::instrument(name = "GET /apps/{app_id}/api", skip(state, user))]
pub async fn get_api_keys(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Path(app_id): Path<String>,
) -> Result<Json<Vec<ApiKeyInfo>>, ApiError> {
    ensure_permission!(user, &app_id, &state, RolePermissions::Admin);

    let technical_users = technical_user::Entity::find()
        .filter(technical_user::Column::AppId.eq(&app_id))
        .limit(1000)
        .all(&state.db)
        .await?;

    // Get all role IDs to fetch role info
    let role_ids: Vec<String> = technical_users
        .iter()
        .filter_map(|tu| tu.role_id.clone())
        .collect();

    let roles: std::collections::HashMap<String, role::Model> = if !role_ids.is_empty() {
        role::Entity::find()
            .filter(role::Column::Id.is_in(role_ids))
            .all(&state.db)
            .await?
            .into_iter()
            .map(|r| (r.id.clone(), r))
            .collect()
    } else {
        std::collections::HashMap::new()
    };

    let api_keys = technical_users
        .into_iter()
        .map(|tu| {
            let role = tu.role_id.as_ref().and_then(|id| roles.get(id));
            ApiKeyInfo {
                id: tu.id,
                name: tu.name,
                description: tu.description,
                role_id: tu.role_id,
                role_name: role.map(|r| r.name.clone()),
                role_permissions: role.map(|r| r.permissions),
                valid_until: tu.valid_until.map(|dt| dt.and_utc().timestamp()),
                created_at: tu.created_at.and_utc().timestamp(),
            }
        })
        .collect();

    Ok(Json(api_keys))
}
