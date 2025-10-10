use std::str::FromStr;

use crate::{entity::pat, error::ApiError, middleware::jwt::AppUser, state::AppState};
use axum::{Extension, Json, extract::State};
use flow_like_types::anyhow;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct PatOut {
    pub name: String,
    pub created_at: i64,
    pub valid_until: Option<i64>,
    pub permissions: i64,
    pub id: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GetPatsOutput {
    pub pats: Vec<PatOut>,
}

#[tracing::instrument(name = "GET /user/pat", skip(state, user))]
pub async fn get_pats(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
) -> Result<Json<Vec<GetPatsOutput>>, ApiError> {
    let sub = user.sub()?;

    let pats = pat::Entity::find()
        .filter(pat::Column::UserId.eq(sub))
        .all(&state.db)
        .await?
        .into_iter()
        .map(|pat| {
            Ok(PatOut {
                name: pat.name,
                created_at: pat.created_at.and_utc().timestamp_millis(),
                valid_until: pat.valid_until.map(|dt| dt.and_utc().timestamp_millis()),
                permissions: pat.permissions,
                id: pat.id,
            })
        })
        .collect::<Result<Vec<PatOut>, ApiError>>()
        .map(|pats| Json(vec![GetPatsOutput { pats }]))?;

    Ok(pats)
}
