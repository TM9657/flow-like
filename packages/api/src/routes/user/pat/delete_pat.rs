use std::str::FromStr;

use crate::{entity::pat, error::ApiError, middleware::jwt::AppUser, state::AppState};
use axum::{Extension, Json, extract::State};
use flow_like_types::anyhow;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct DeletePatInput {
    pub id: String,
}

#[tracing::instrument(name = "DELETE /user/pat", skip(state, user, input))]
pub async fn delete_pat(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Json(input): Json<DeletePatInput>,
) -> Result<Json<()>, ApiError> {
    let sub = user.sub()?;

    pat::Entity::delete_many()
        .filter(pat::Column::Id.eq(input.id).and(pat::Column::UserId.eq(sub)))
        .exec(&state.db)
        .await?;

    Ok(Json(()))
}
