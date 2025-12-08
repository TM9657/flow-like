use crate::{
    error::ApiError, middleware::jwt::AppUser, permission::global_permission::GlobalPermission,
    state::AppState,
};
use axum::{
    Extension, Json,
    extract::{Path, State},
};
use flow_like_types::anyhow;
use sea_orm::EntityTrait;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SolutionDetail {
    pub id: String,
    pub name: String,
    pub email: String,
    pub company: String,
    pub description: String,
    pub application_type: String,
    pub data_security: String,
    pub example_input: String,
    pub expected_output: String,
    pub user_count: String,
    pub user_type: String,
    pub technical_level: String,
    pub timeline: Option<String>,
    pub additional_notes: Option<String>,
    pub pricing_tier: String,
    pub paid_deposit: bool,
    pub files: Option<serde_json::Value>,
    pub storage_key: Option<String>,
    pub status: String,
    pub stripe_checkout_session_id: Option<String>,
    pub stripe_payment_intent_id: Option<String>,
    pub stripe_setup_intent_id: Option<String>,
    pub total_cents: i64,
    pub deposit_cents: i64,
    pub remainder_cents: i64,
    pub priority: bool,
    pub admin_notes: Option<String>,
    pub assigned_to: Option<String>,
    pub delivered_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[tracing::instrument(name = "GET /admin/solutions/{solution_id}", skip(state, user))]
pub async fn get_solution(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Path(solution_id): Path<String>,
) -> Result<Json<SolutionDetail>, ApiError> {
    use crate::entity::solution_request;

    user.check_global_permission(&state, GlobalPermission::ReadSolutions)
        .await?;

    let solution = solution_request::Entity::find_by_id(&solution_id)
        .one(&state.db)
        .await?
        .ok_or_else(|| anyhow!("Solution request not found"))?;

    let detail = SolutionDetail {
        id: solution.id,
        name: solution.name,
        email: solution.email,
        company: solution.company,
        description: solution.description,
        application_type: solution.application_type,
        data_security: solution.data_security,
        example_input: solution.example_input,
        expected_output: solution.expected_output,
        user_count: solution.user_count,
        user_type: solution.user_type,
        technical_level: solution.technical_level,
        timeline: solution.timeline,
        additional_notes: solution.additional_notes,
        pricing_tier: format!("{:?}", solution.pricing_tier).to_lowercase(),
        paid_deposit: solution.paid_deposit,
        files: solution.files,
        storage_key: solution.storage_key,
        status: format!("{:?}", solution.status),
        stripe_checkout_session_id: solution.stripe_checkout_session_id,
        stripe_payment_intent_id: solution.stripe_payment_intent_id,
        stripe_setup_intent_id: solution.stripe_setup_intent_id,
        total_cents: solution.total_cents,
        deposit_cents: solution.deposit_cents,
        remainder_cents: solution.remainder_cents,
        priority: solution.priority,
        admin_notes: solution.admin_notes,
        assigned_to: solution.assigned_to,
        delivered_at: solution.delivered_at.map(|d| d.to_string()),
        created_at: solution.created_at.to_string(),
        updated_at: solution.updated_at.to_string(),
    };

    Ok(Json(detail))
}
