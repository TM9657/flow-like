use crate::{entity::user, error::ApiError, middleware::jwt::AppUser, state::AppState};
use axum::{Extension, Json, extract::State};
use sea_orm::EntityTrait;
use serde::Serialize;
use utoipa::ToSchema;

/// Usage statistics for LLM and embedding consumption
#[derive(Serialize, Debug, Clone, ToSchema)]
pub struct Usage {
    /// Total LLM consumption in micro-dollars
    pub llm_price: i64,
    /// Total embedding consumption in micro-dollars
    pub embedding_price: i64,
}

/// Get combined usage statistics for LLM and embeddings
#[utoipa::path(
    get,
    path = "/chat",
    tag = "chat",
    responses(
        (status = 200, description = "LLM and embedding usage statistics", body = Usage)
    )
)]
#[tracing::instrument(name = "GET /usage", skip(state, user))]
pub async fn get_llm_usage(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
) -> Result<Json<Usage>, ApiError> {
    if !state.platform_config.features.unauthorized_read {
        user.sub()?;
    }

    let user = user::Entity::find_by_id(user.sub()?)
        .one(&state.db)
        .await?
        .ok_or(ApiError::FORBIDDEN)?;

    Ok(Json(Usage {
        llm_price: user.total_llm_price,
        embedding_price: user.total_embedding_price,
    }))
}
