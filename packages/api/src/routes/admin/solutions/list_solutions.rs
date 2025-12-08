use crate::{
    error::ApiError, middleware::jwt::AppUser, permission::global_permission::GlobalPermission,
    state::AppState,
};
use axum::{Extension, Json, extract::State};
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SolutionListItem {
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
    pub status: String,
    pub priority: bool,
    pub paid_deposit: bool,
    pub files: Option<serde_json::Value>,
    pub storage_key: Option<String>,
    pub total_cents: i64,
    pub deposit_cents: i64,
    pub remainder_cents: i64,
    pub assigned_to: Option<String>,
    pub admin_notes: Option<String>,
    pub delivered_at: Option<String>,
    pub tracking_token: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ListSolutionsResponse {
    pub solutions: Vec<SolutionListItem>,
    pub total: u64,
    pub page: u64,
    pub limit: u64,
    pub has_more: bool,
}

#[derive(Clone, Deserialize, Debug)]
pub struct ListSolutionsQuery {
    pub status: Option<String>,
    pub search: Option<String>,
    pub page: Option<u64>,
    pub limit: Option<u64>,
}

#[tracing::instrument(name = "GET /admin/solutions", skip(state, user))]
pub async fn list_solutions(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    axum::extract::Query(query): axum::extract::Query<ListSolutionsQuery>,
) -> Result<Json<ListSolutionsResponse>, ApiError> {
    use crate::entity::solution_request;

    user.check_global_permission(&state, GlobalPermission::ReadSolutions)
        .await?;

    let page = query.page.unwrap_or(1).max(1);
    let limit = query.limit.unwrap_or(25).min(100);
    let offset = (page - 1) * limit;

    let mut select =
        solution_request::Entity::find().order_by_desc(solution_request::Column::CreatedAt);

    if let Some(status_filter) = &query.status {
        let status = match status_filter.to_lowercase().as_str() {
            "pending_payment" => {
                crate::entity::sea_orm_active_enums::SolutionStatus::PendingPayment
            }
            "pending_review" => crate::entity::sea_orm_active_enums::SolutionStatus::PendingReview,
            "in_progress" => crate::entity::sea_orm_active_enums::SolutionStatus::InProgress,
            "delivered" => crate::entity::sea_orm_active_enums::SolutionStatus::Delivered,
            "cancelled" => crate::entity::sea_orm_active_enums::SolutionStatus::Cancelled,
            "refunded" => crate::entity::sea_orm_active_enums::SolutionStatus::Refunded,
            _ => return Err(ApiError::BadRequest("Invalid status filter".to_string())),
        };
        select = select.filter(solution_request::Column::Status.eq(status));
    }

    if let Some(search) = &query.search {
        if !search.trim().is_empty() {
            let search_pattern = format!("%{}%", search.trim().to_lowercase());
            select = select.filter(
                solution_request::Column::Name
                    .like(&search_pattern)
                    .or(solution_request::Column::Email.like(&search_pattern))
                    .or(solution_request::Column::Company.like(&search_pattern)),
            );
        }
    }

    let total = select.clone().count(&state.db).await?;

    let solutions = select
        .paginate(&state.db, limit)
        .fetch_page(offset / limit.max(1))
        .await?;

    let items: Vec<SolutionListItem> = solutions
        .into_iter()
        .map(|s| SolutionListItem {
            id: s.id,
            name: s.name,
            email: s.email,
            company: s.company,
            description: s.description,
            application_type: s.application_type,
            data_security: s.data_security,
            example_input: s.example_input,
            expected_output: s.expected_output,
            user_count: s.user_count,
            user_type: s.user_type,
            technical_level: s.technical_level,
            timeline: s.timeline,
            additional_notes: s.additional_notes,
            pricing_tier: format!("{:?}", s.pricing_tier).to_lowercase(),
            status: format!("{:?}", s.status),
            priority: s.priority,
            paid_deposit: s.paid_deposit,
            files: s.files,
            storage_key: s.storage_key,
            total_cents: s.total_cents,
            deposit_cents: s.deposit_cents,
            remainder_cents: s.remainder_cents,
            assigned_to: s.assigned_to,
            admin_notes: s.admin_notes,
            delivered_at: s.delivered_at.map(|d| d.to_string()),
            tracking_token: s.tracking_token,
            created_at: s.created_at.to_string(),
            updated_at: s.updated_at.to_string(),
        })
        .collect();

    let has_more = (page * limit) < total;

    Ok(Json(ListSolutionsResponse {
        solutions: items,
        total,
        page,
        limit,
        has_more,
    }))
}
