use crate::{
    error::ApiError, middleware::jwt::AppUser, permission::global_permission::GlobalPermission,
    state::AppState,
};
use axum::{
    Extension, Json,
    extract::{Path, State},
};
use flow_like_types::{anyhow, create_id};
use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait};
use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AddLogBody {
    pub action: String,
    pub details: Option<String>,
}

#[derive(Clone, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AddLogResponse {
    pub success: bool,
    pub log_id: String,
}

#[tracing::instrument(
    name = "POST /admin/solutions/{solution_id}/logs",
    skip(state, user, body)
)]
pub async fn add_solution_log(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Path(solution_id): Path<String>,
    Json(body): Json<AddLogBody>,
) -> Result<Json<AddLogResponse>, ApiError> {
    use crate::entity::{solution_log, solution_request};

    user.check_global_permission(&state, GlobalPermission::WriteSolutions)
        .await?;

    // Verify solution exists
    solution_request::Entity::find_by_id(&solution_id)
        .one(&state.db)
        .await?
        .ok_or_else(|| anyhow!("Solution request not found"))?;

    let log_id = create_id();
    let actor = user.sub().ok();

    let new_log = solution_log::ActiveModel {
        id: Set(log_id.clone()),
        solution_id: Set(solution_id.clone()),
        action: Set(body.action),
        details: Set(body.details),
        actor: Set(actor),
        created_at: Set(chrono::Utc::now().naive_utc()),
    };

    new_log.insert(&state.db).await?;

    tracing::info!(
        solution_id = %solution_id,
        log_id = %log_id,
        "Solution log added by admin"
    );

    Ok(Json(AddLogResponse {
        success: true,
        log_id,
    }))
}
