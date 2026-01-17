use crate::{entity::user, error::ApiError, middleware::jwt::AppUser, state::AppState};
use axum::{Extension, Json, extract::State};
use flow_like::flow_like_model_provider::history::History;
use sea_orm::EntityTrait;
use serde::Serialize;

#[derive(Serialize, Debug, Clone)]
pub struct Usage {
    pub llm_price: i64,
}

#[tracing::instrument(name = "GET /llm", skip(state, user))]
pub async fn get_llm_usage(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Json(history): Json<History>,
) -> Result<Json<Usage>, ApiError> {
    if !state.platform_config.features.unauthorized_read {
        user.sub()?;
    }

    let user = user::Entity::find_by_id(user.sub()?)
        .one(&state.db)
        .await?
        .ok_or_else(|| ApiError::FORBIDDEN)?;

    Ok(Json(Usage {
        llm_price: user.total_llm_price,
    }))
}
