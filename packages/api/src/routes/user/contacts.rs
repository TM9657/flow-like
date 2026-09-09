use crate::{
    entity::{invitation, membership, role, sea_orm_active_enums::UserStatus, user},
    error::ApiError,
    middleware::jwt::AppUser,
    permission::role_permission::RolePermissions,
    routes::user::lookup::UserLookupResponse,
    state::AppState,
};
use axum::{
    Extension, Json,
    extract::{Query, State},
};
use futures::stream::{self, StreamExt};
use sea_orm::{
    ColumnTrait, Condition, EntityTrait, JoinType, QueryFilter, QueryOrder, QuerySelect,
    QueryTrait, RelationTrait,
    sea_query::{Expr, ExprTrait},
};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

const MAX_CONTACTS_PAGE_SIZE: u64 = 500;
const AVATAR_CONCURRENCY: usize = 16;

#[derive(Debug, Deserialize, IntoParams)]
pub struct UserContactsQuery {
    /// Project receiving the invitations.
    pub app_id: String,
    /// Last user ID from the preceding page.
    pub after: Option<String>,
    /// Page size, between 1 and 500. Defaults to 500.
    pub limit: Option<u64>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UserContactsResponse {
    pub users: Vec<UserLookupResponse>,
    pub next_cursor: Option<String>,
}

/// Select users through membership subqueries so multiple shared projects cannot
/// duplicate a contact or consume page slots. Permissions apply before paging.
fn contacts_query(
    caller_id: &str,
    target_app_id: &str,
    after: Option<&str>,
    limit: u64,
) -> sea_orm::Select<user::Entity> {
    let readable_permissions =
        RolePermissions::ReadTeam | RolePermissions::Admin | RolePermissions::Owner;
    let readable_projects = membership::Entity::find()
        .select_only()
        .column(membership::Column::AppId)
        .join(JoinType::InnerJoin, membership::Relation::Role.def())
        .filter(membership::Column::UserId.eq(caller_id))
        .filter(membership::Column::AppId.ne(target_app_id))
        .filter(
            Expr::col((role::Entity, role::Column::Permissions))
                .bit_and(readable_permissions.bits())
                .ne(0),
        )
        // Match app_permission's rejection of invalid permission bitfields.
        .filter(
            Expr::col((role::Entity, role::Column::Permissions))
                .bit_and(!RolePermissions::all().bits())
                .eq(0),
        )
        .into_query();

    let shared_project_users = membership::Entity::find()
        .select_only()
        .column(membership::Column::UserId)
        .filter(membership::Column::AppId.in_subquery(readable_projects))
        .into_query();

    let mut query = user::Entity::find()
        .filter(user::Column::Id.in_subquery(shared_project_users))
        .filter(invitation_candidates(target_app_id, caller_id))
        .filter(user::Column::Status.ne(UserStatus::Banned));

    if let Some(after) = after {
        query = query.filter(user::Column::Id.gt(after));
    }

    query
        .order_by_asc(user::Column::Id)
        .limit(limit.clamp(1, MAX_CONTACTS_PAGE_SIZE) + 1)
}

/// Apply invitation exclusions before either contacts or directory search is capped.
pub(super) fn invitation_candidates(target_app_id: &str, caller_id: &str) -> Condition {
    let target_members = membership::Entity::find()
        .select_only()
        .column(membership::Column::UserId)
        .filter(membership::Column::AppId.eq(target_app_id))
        .into_query();
    // Invitations are deleted on acceptance or rejection, so every row is pending.
    let target_invitees = invitation::Entity::find()
        .select_only()
        .column(invitation::Column::UserId)
        .filter(invitation::Column::AppId.eq(target_app_id))
        .into_query();

    Condition::all()
        .add(user::Column::Id.not_in_subquery(target_members))
        .add(user::Column::Id.not_in_subquery(target_invitees))
        .add(user::Column::Id.ne(caller_id))
}

#[utoipa::path(
    get,
    path = "/user/contacts",
    tag = "user",
    params(UserContactsQuery),
    responses(
        (status = 200, description = "Users from other readable project teams, excluding existing members and invitees", body = UserContactsResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Requires a user session and permission to invite to the target project")
    ),
    security(
        ("bearer_auth" = []),
        ("pat" = [])
    )
)]
#[tracing::instrument(name = "GET /user/contacts", skip_all)]
pub async fn user_contacts(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Query(params): Query<UserContactsQuery>,
) -> Result<Json<UserContactsResponse>, ApiError> {
    // App-scoped principals cannot use a passed-through subject to read other teams.
    let caller_id = user.sub()?;
    let permission = user.app_permission(&params.app_id, &state).await?;
    if !permission.has_permission(RolePermissions::Admin) {
        return Err(ApiError::FORBIDDEN);
    }

    let limit = params
        .limit
        .unwrap_or(MAX_CONTACTS_PAGE_SIZE)
        .clamp(1, MAX_CONTACTS_PAGE_SIZE);
    let mut contacts = contacts_query(&caller_id, &params.app_id, params.after.as_deref(), limit)
        .all(&state.db)
        .await?;
    let has_more = contacts.len() > limit as usize;
    contacts.truncate(limit as usize);
    let next_cursor = has_more
        .then(|| contacts.last().map(|contact| contact.id.clone()))
        .flatten();

    let lookup_config = state.platform_config.lookup.clone();
    let users = stream::iter(contacts)
        .map(|contact| UserLookupResponse::parse(contact, lookup_config.clone(), &state))
        .buffered(AVATAR_CONCURRENCY)
        .collect()
        .await;

    Ok(Json(UserContactsResponse { users, next_cursor }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::DbBackend;

    fn sql(after: Option<&str>, limit: u64) -> String {
        contacts_query("caller", "target", after, limit)
            .build(DbBackend::Postgres)
            .to_string()
    }

    #[test]
    fn contacts_require_membership_and_team_access_in_other_projects() {
        let sql = sql(None, 500);
        assert!(sql.contains(r#""Membership"."userId" = 'caller'"#));
        assert!(sql.contains(r#""Membership"."appId" <> 'target'"#));
        assert!(sql.contains(r#"INNER JOIN "public"."Role""#));
        assert!(sql.contains(r#""Role"."permissions" & 7) <> 0"#));
        assert!(sql.find("permissions") < sql.find("LIMIT"));
    }

    #[test]
    fn contacts_exclude_self_banned_users_members_and_pending_invitees() {
        let sql = sql(None, 500);
        assert!(sql.contains(r#""User"."id" <> 'caller'"#));
        assert!(sql.contains(r#""User"."status" <> 'BANNED'"#));
        assert_eq!(sql.matches("NOT IN (SELECT").count(), 2);
        assert!(sql.contains(r#""Membership"."appId" = 'target'"#));
        assert!(sql.contains(r#""Invitation"."appId" = 'target'"#));
    }

    #[test]
    fn pages_use_exclusive_user_id_cursor_with_one_lookahead_row() {
        let sql = sql(Some("last-user"), 100);
        assert!(sql.contains(r#""User"."id" > 'last-user'"#));
        assert!(sql.ends_with(r#"ORDER BY "User"."id" ASC LIMIT 101"#));
        assert!(!sql.contains("OFFSET"));
    }

    #[test]
    fn page_size_is_bounded_even_for_untrusted_limits() {
        assert!(sql(None, 0).ends_with("LIMIT 2"));
        assert!(sql(None, u64::MAX).ends_with("LIMIT 501"));
    }
}
