use crate::{
    error::ApiError, middleware::jwt::AppUser, permission::global_permission::GlobalPermission,
    state::AppState,
};
use axum::{
    Extension, Json,
    extract::{Path, State},
};
use flow_like_types::anyhow;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait, IntoActiveModel};
use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSolutionBody {
    pub status: Option<String>,
    pub admin_notes: Option<String>,
    pub assigned_to: Option<String>,
    pub priority: Option<bool>,
}

#[derive(Clone, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSolutionResponse {
    pub success: bool,
    pub id: String,
    pub new_status: String,
}

#[tracing::instrument(name = "PATCH /admin/solutions/{solution_id}", skip(state, user, body))]
pub async fn update_solution(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Path(solution_id): Path<String>,
    Json(body): Json<UpdateSolutionBody>,
) -> Result<Json<UpdateSolutionResponse>, ApiError> {
    use crate::entity::{sea_orm_active_enums::SolutionStatus, solution_request};

    user.check_global_permission(&state, GlobalPermission::WriteSolutions)
        .await?;

    let solution = solution_request::Entity::find_by_id(&solution_id)
        .one(&state.db)
        .await?
        .ok_or_else(|| anyhow!("Solution request not found"))?;

    let mut active: solution_request::ActiveModel = solution.into_active_model();

    if let Some(status_str) = &body.status {
        let new_status = match status_str.to_lowercase().as_str() {
            "awaiting_deposit" => SolutionStatus::AwaitingDeposit,
            "pending_review" => SolutionStatus::PendingReview,
            "in_queue" => SolutionStatus::InQueue,
            "onboarding_done" => SolutionStatus::OnboardingDone,
            "in_progress" => SolutionStatus::InProgress,
            "delivered" => SolutionStatus::Delivered,
            "awaiting_payment" => SolutionStatus::AwaitingPayment,
            "paid" => SolutionStatus::Paid,
            "cancelled" => SolutionStatus::Cancelled,
            "refunded" => SolutionStatus::Refunded,
            _ => return Err(ApiError::BadRequest("Invalid status".to_string())),
        };
        active.status = Set(new_status.clone());

        if new_status == SolutionStatus::Delivered {
            active.delivered_at = Set(Some(chrono::Utc::now().naive_utc()));
        }
    }

    if let Some(notes) = body.admin_notes {
        active.admin_notes = Set(Some(notes));
    }

    if let Some(assigned) = body.assigned_to {
        active.assigned_to = Set(Some(assigned));
    }

    if let Some(priority) = body.priority {
        active.priority = Set(priority);
    }

    active.updated_at = Set(chrono::Utc::now().naive_utc());

    let updated = active.update(&state.db).await?;

    tracing::info!(
        solution_id = %solution_id,
        new_status = ?updated.status,
        "Solution request updated by admin"
    );

    Ok(Json(UpdateSolutionResponse {
        success: true,
        id: solution_id,
        new_status: status_to_string(&updated.status),
    }))
}

fn status_to_string(status: &crate::entity::sea_orm_active_enums::SolutionStatus) -> String {
    use crate::entity::sea_orm_active_enums::SolutionStatus;
    match status {
        SolutionStatus::AwaitingDeposit => "AWAITING_DEPOSIT".to_string(),
        SolutionStatus::PendingReview => "PENDING_REVIEW".to_string(),
        SolutionStatus::InQueue => "IN_QUEUE".to_string(),
        SolutionStatus::OnboardingDone => "ONBOARDING_DONE".to_string(),
        SolutionStatus::InProgress => "IN_PROGRESS".to_string(),
        SolutionStatus::Delivered => "DELIVERED".to_string(),
        SolutionStatus::AwaitingPayment => "AWAITING_PAYMENT".to_string(),
        SolutionStatus::Paid => "PAID".to_string(),
        SolutionStatus::Cancelled => "CANCELLED".to_string(),
        SolutionStatus::Refunded => "REFUNDED".to_string(),
    }
}
