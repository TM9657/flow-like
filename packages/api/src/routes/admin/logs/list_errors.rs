//! List and filter API error reports for the admin control tower.

use crate::entity::error_report;
use crate::error::ApiError;
use crate::middleware::jwt::AppUser;
use crate::permission::global_permission::GlobalPermission;
use crate::state::AppState;
use axum::extract::{Query, State};
use axum::{Extension, Json};
use chrono::{DateTime, Duration, FixedOffset, Utc};
use sea_orm::{
    ColumnTrait, Condition, EntityTrait, Order, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect,
};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Deserialize, IntoParams)]
pub struct ListErrorsQuery {
    /// Free text search applied to id, summary, public code, path, user id.
    #[serde(default)]
    pub query: Option<String>,
    /// Filter by exact error id (e.g. references shared with users).
    #[serde(default)]
    pub error_id: Option<String>,
    /// Filter by HTTP method.
    #[serde(default)]
    pub method: Option<String>,
    /// Filter by exact path.
    #[serde(default)]
    pub path: Option<String>,
    /// Filter by exact public code.
    #[serde(default)]
    pub public_code: Option<String>,
    /// Filter by status code.
    #[serde(default)]
    pub status_code: Option<i32>,
    /// Filter by associated user id.
    #[serde(default)]
    pub user_id: Option<String>,
    /// Severity bucket: "client" (4xx), "server" (5xx). Anything else is ignored.
    #[serde(default)]
    pub severity: Option<String>,
    /// Lookback window in hours. Default 24.
    #[serde(default)]
    pub hours: Option<i64>,
    #[serde(default)]
    pub offset: Option<u64>,
    #[serde(default)]
    pub limit: Option<u64>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorReportRecord {
    pub id: String,
    pub user_id: Option<String>,
    pub method: String,
    pub path: String,
    pub status_code: i32,
    pub public_code: String,
    pub summary: String,
    pub details: Option<serde_json::Value>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ListErrorsResponse {
    pub errors: Vec<ErrorReportRecord>,
    pub total: u64,
    pub offset: u64,
    pub limit: u64,
}

impl From<error_report::Model> for ErrorReportRecord {
    fn from(m: error_report::Model) -> Self {
        Self {
            id: m.id,
            user_id: m.user_id,
            method: m.method,
            path: m.path,
            status_code: m.status_code,
            public_code: m.public_code,
            summary: m.summary,
            details: m.details,
            created_at: m.created_at.to_rfc3339(),
            updated_at: m.updated_at.to_rfc3339(),
        }
    }
}

fn cutoff(hours: Option<i64>) -> Result<Option<DateTime<FixedOffset>>, ApiError> {
    let h = hours.unwrap_or(24).max(0);
    if h == 0 {
        Ok(None)
    } else {
        let duration = Duration::try_hours(h)
            .ok_or_else(|| ApiError::bad_request("hours is outside the supported time range"))?;
        let cutoff = Utc::now()
            .checked_sub_signed(duration)
            .ok_or_else(|| ApiError::bad_request("hours is outside the supported time range"))?;
        Ok(Some(cutoff.fixed_offset()))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn cutoff_rejects_overflow_without_changing_normal_windows() {
        assert!(super::cutoff(Some(i64::MAX)).is_err());
        assert!(super::cutoff(Some(10_000_000_000)).is_err());
        assert_eq!(super::cutoff(Some(0)).unwrap(), None);
        assert_eq!(super::cutoff(Some(-1)).unwrap(), None);
        let cutoff = super::cutoff(Some(24)).unwrap().unwrap();
        assert!(
            (chrono::Utc::now().fixed_offset() - cutoff)
                .num_seconds()
                .abs_diff(86_400)
                < 2
        );
        assert!(super::cutoff(None).unwrap().is_some());
    }
}

#[utoipa::path(
    get,
    path = "/admin/logs/errors",
    tag = "admin",
    params(ListErrorsQuery),
    responses(
        (status = 200, description = "Paginated list of recent error reports", body = ListErrorsResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    description = "List and filter API error reports. Requires ReadLogs permission."
)]
pub async fn list_errors(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Query(q): Query<ListErrorsQuery>,
) -> Result<Json<ListErrorsResponse>, ApiError> {
    use sea_orm::sea_query::ExprTrait;
    user.check_global_permission(&state, GlobalPermission::ReadLogs)
        .await?;

    let offset = q.offset.unwrap_or(0);
    let limit = Ord::min(q.limit.unwrap_or(50), 200);

    let mut select = error_report::Entity::find();

    if let Some(cutoff) = cutoff(q.hours)? {
        select = select.filter(error_report::Column::CreatedAt.gte(cutoff));
    }

    if let Some(id) = &q.error_id
        && !id.is_empty()
    {
        select = select.filter(error_report::Column::Id.eq(id));
    }

    if let Some(method) = &q.method
        && !method.is_empty()
    {
        select = select.filter(error_report::Column::Method.eq(method.to_uppercase()));
    }

    if let Some(path) = &q.path
        && !path.is_empty()
    {
        select = select.filter(error_report::Column::Path.eq(path));
    }

    if let Some(code) = &q.public_code
        && !code.is_empty()
    {
        select = select.filter(error_report::Column::PublicCode.eq(code));
    }

    if let Some(sc) = q.status_code {
        select = select.filter(error_report::Column::StatusCode.eq(sc));
    }

    if let Some(uid) = &q.user_id
        && !uid.is_empty()
    {
        select = select.filter(error_report::Column::UserId.eq(uid));
    }

    match q.severity.as_deref() {
        Some("client") => {
            select = select.filter(
                error_report::Column::StatusCode
                    .gte(400)
                    .and(error_report::Column::StatusCode.lt(500)),
            );
        }
        Some("server") => {
            select = select.filter(error_report::Column::StatusCode.gte(500));
        }
        _ => {}
    }

    if let Some(text) = &q.query
        && !text.is_empty()
    {
        let pattern = format!("%{}%", text);
        select = select.filter(
            Condition::any()
                .add(error_report::Column::Id.like(&pattern))
                .add(error_report::Column::Summary.like(&pattern))
                .add(error_report::Column::PublicCode.like(&pattern))
                .add(error_report::Column::Path.like(&pattern))
                .add(error_report::Column::UserId.like(&pattern)),
        );
    }

    let total = select.clone().count(&state.db).await?;

    let records = select
        .order_by(error_report::Column::CreatedAt, Order::Desc)
        .offset(offset)
        .limit(limit)
        .all(&state.db)
        .await?;

    Ok(Json(ListErrorsResponse {
        errors: records.into_iter().map(Into::into).collect(),
        total,
        offset,
        limit,
    }))
}
