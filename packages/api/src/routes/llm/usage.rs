use crate::{entity::template_profile, error::ApiError, middleware::jwt::AppUser, state::AppState};
use axum::{Extension, Json, extract::State};
use flow_like::profile::{Profile, Settings};
use sea_orm::EntityTrait;

#[tracing::instrument(name = "GET /llm", skip(state, user))]
pub async fn get_llm_usage(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Json(history): Json<History>
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, ApiError> {
    if !state.platform_config.features.unauthorized_read {
        user.sub()?;
    }

    let profiles = template_profile::Entity::find().all(&state.db).await?;
    let profiles: Vec<Profile> = profiles.into_iter().map(Profile::from).collect();

    Ok(Json(profiles))
}