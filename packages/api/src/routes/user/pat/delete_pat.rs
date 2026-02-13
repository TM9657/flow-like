use crate::{entity::pat, error::ApiError, middleware::jwt::AppUser, state::AppState};
use axum::{Extension, Json, extract::State};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct DeletePatInput {
    pub id: String,
}

#[utoipa::path(
    delete,
    path = "/user/pat",
    tag = "user",
    request_body = DeletePatInput,
    responses(
        (status = 200, description = "Personal access token deleted"),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
#[tracing::instrument(name = "DELETE /user/pat", skip(state, user, input))]
pub async fn delete_pat(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Json(input): Json<DeletePatInput>,
) -> Result<Json<()>, ApiError> {
    let sub = user.sub()?;

    pat::Entity::delete_many()
        .filter(
            pat::Column::Id
                .eq(input.id)
                .and(pat::Column::UserId.eq(sub)),
        )
        .exec(&state.db)
        .await?;

    Ok(Json(()))
}
