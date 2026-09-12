//! Scheduled events across every app the signed-in user can read events for.
//!
//! Without this, a caller wanting "my upcoming schedules" has to ask
//! `/apps/{app_id}/events` once per app and discard everything that is not a
//! cron event — a fan-out that grows with the size of the library and repeats
//! on every refresh. One membership join answers the same question once.
//!
//! Only the database mirror of the event table is read. `GET /apps/{id}/events`
//! additionally rebuilds the mirror from app storage when an app has no rows at
//! all, which is how a legacy or interrupted-sync app recovers; an app in that
//! state contributes nothing here until something reads its events directly.

use crate::{
    entity::{app, event, membership, role, sea_orm_active_enums::Status},
    error::ApiError,
    middleware::jwt::AppUser,
    permission::role_permission::{RolePermissions, has_role_permission},
    state::AppState,
};
use axum::{
    Extension, Json,
    extract::{Query, State},
};
use base64::Engine;
use sea_orm::{
    ColumnTrait, EntityTrait, JoinType, QueryFilter, QueryOrder, QuerySelect, RelationTrait,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::{IntoParams, ToSchema};

/// Event type recorded for schedule-triggered events.
const CRON_EVENT_TYPE: &str = "cron";
const DEFAULT_LIMIT: u64 = 100;
const MAX_LIMIT: u64 = 500;
/// Memberships scanned when resolving which apps the caller can read events for.
const MAX_MEMBERSHIPS: u64 = 500;

#[derive(Clone, Debug, Deserialize, IntoParams)]
pub struct ScheduleParams {
    /// Maximum schedules to return. Clamped to 500.
    pub limit: Option<u64>,
    /// Restrict to one app. Omit for every app the caller can list.
    pub app_id: Option<String>,
}

/// The schedule half of an event's configuration, and nothing else.
///
/// An event config is a single JSON blob that also carries transport secrets —
/// `auth_token` for HTTP-triggered events — so it is projected down to the
/// fields a schedule preview needs rather than returned whole.
#[derive(Clone, Debug, Default, PartialEq, Serialize, ToSchema)]
pub struct ScheduleConfig {
    /// Cron expression, under whichever key the event recorded it.
    pub expression: Option<String>,
    pub timezone: Option<String>,
    /// `{ "date": "YYYY-MM-DD", "time": "HH:MM" }` for a one-shot schedule.
    pub scheduled_for: Option<Value>,
    pub last_fired: Option<String>,
}

#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct UserSchedule {
    pub event_id: String,
    pub app_id: String,
    pub name: String,
    pub description: Option<String>,
    /// The next fire time is derived from this by the caller, which owns the
    /// calendar rules; the event's own runner remains the only thing that fires.
    pub config: ScheduleConfig,
}

#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct UserSchedulesResponse {
    pub schedules: Vec<UserSchedule>,
    /// Apps the caller can read events for that were considered.
    pub apps_checked: usize,
    /// Schedule rows whose configuration could not be read.
    pub unreadable: usize,
    /// True when `limit` cut the list short.
    pub truncated: bool,
}

/// The mirror stores an event config as `{"base64": "<bytes>"}`; those bytes are
/// the config JSON.
fn decode_config(stored: Option<&Value>) -> Option<Value> {
    let encoded = stored?.get("base64")?.as_str()?;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .ok()?;
    serde_json::from_slice(&bytes).ok()
}

fn first_string(config: &Value, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| config.get(*key)?.as_str())
        .map(ToOwned::to_owned)
}

/// Keep the aliases the sink layer accepts, but hand the caller one spelling.
fn project_schedule(config: &Value) -> ScheduleConfig {
    ScheduleConfig {
        expression: first_string(
            config,
            &[
                "expression",
                "cron_expression",
                "cronExpression",
                "cron",
                "schedule",
            ],
        ),
        timezone: first_string(config, &["timezone", "tz", "cron_timezone", "cronTimezone"]),
        scheduled_for: config
            .get("scheduled_for")
            .or_else(|| config.get("scheduledFor"))
            .filter(|value| value.get("date").is_some() && value.get("time").is_some())
            .cloned(),
        last_fired: first_string(config, &["last_fired", "lastFired"]),
    }
}

/// Apps whose role lets the caller read this app's events, enforced against the
/// stored role bits rather than over an already-fetched list.
///
/// `ReadEvents`, not `ListEvents`. `GET /apps/{id}/events` hands a
/// `ListEvents`-only caller just the user-facing event types — `simple_chat`,
/// `generic_form`, `quick_action`, or anything carrying a default page — and a
/// cron event is none of those. Gating here on `ListEvents` would show schedules
/// to viewers who cannot see them through the per-app endpoint.
///
/// The app id comes from the membership row: `Role.appId` is nullable, because a
/// global or template role belongs to no app, so selecting it would fail to
/// decode the moment such a role is attached.
///
/// `INACTIVE` apps are excluded for the same reason the library excludes them:
/// the status marks a deletion tombstone or the destination of an in-flight
/// fork, neither of which should contribute a schedule.
async fn listable_app_ids(
    state: &AppState,
    user_id: &str,
    only: Option<&str>,
) -> Result<Vec<String>, ApiError> {
    let mut query = membership::Entity::find()
        .select_only()
        .column(membership::Column::AppId)
        .column(role::Column::Permissions)
        .join(JoinType::InnerJoin, membership::Relation::Role.def())
        .join(JoinType::InnerJoin, membership::Relation::App.def())
        .filter(membership::Column::UserId.eq(user_id))
        .filter(app::Column::Status.ne(Status::Inactive));

    if let Some(wanted) = only {
        query = query.filter(membership::Column::AppId.eq(wanted));
    }

    let rows = query
        .order_by_desc(membership::Column::UpdatedAt)
        .limit(Some(MAX_MEMBERSHIPS))
        .into_tuple::<(String, i64)>()
        .all(&state.db)
        .await?;

    Ok(rows
        .into_iter()
        .filter_map(|(app_id, permissions)| {
            let permission = RolePermissions::from_bits(permissions)?;
            has_role_permission(&permission, RolePermissions::ReadEvents).then_some(app_id)
        })
        .collect())
}

#[utoipa::path(
    get,
    path = "/user/schedules",
    tag = "user",
    description = "List active scheduled events across every app whose events you can read.",
    params(ScheduleParams),
    responses(
        (status = 200, description = "Active schedules for the signed-in user", body = UserSchedulesResponse),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
#[tracing::instrument(name = "GET /user/schedules", skip_all)]
pub async fn get_schedules(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Query(params): Query<ScheduleParams>,
) -> Result<Json<UserSchedulesResponse>, ApiError> {
    let user_id = user.sub()?;
    let limit = params.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let app_ids = listable_app_ids(&state, &user_id, params.app_id.as_deref()).await?;

    if app_ids.is_empty() {
        return Ok(Json(UserSchedulesResponse {
            schedules: Vec::new(),
            apps_checked: 0,
            unreadable: 0,
            truncated: false,
        }));
    }

    // One row over is how a full page is told apart from a page that happens to
    // land exactly on the limit.
    let rows = event::Entity::find()
        .filter(event::Column::AppId.is_in(app_ids.clone()))
        .filter(event::Column::EventType.eq(CRON_EVENT_TYPE))
        .filter(event::Column::Active.eq(true))
        .order_by_asc(event::Column::AppId)
        .order_by_asc(event::Column::Id)
        .limit(limit + 1)
        .all(&state.db)
        .await?;

    let truncated = rows.len() as u64 > limit;
    let mut unreadable = 0;
    let schedules = rows
        .into_iter()
        .take(limit as usize)
        .filter_map(|row| {
            let Some(config) = decode_config(row.config.as_ref()) else {
                unreadable += 1;
                return None;
            };
            Some(UserSchedule {
                event_id: row.id,
                app_id: row.app_id,
                name: row.name,
                description: row.description,
                config: project_schedule(&config),
            })
        })
        .collect();

    Ok(Json(UserSchedulesResponse {
        schedules,
        apps_checked: app_ids.len(),
        unreadable,
        truncated,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn stored(config: Value) -> Value {
        let bytes = serde_json::to_vec(&config).expect("serializable");
        json!({ "base64": base64::engine::general_purpose::STANDARD.encode(bytes) })
    }

    #[test]
    fn a_stored_config_round_trips_through_base64() {
        let value = stored(json!({ "expression": "0 9 * * *" }));
        assert_eq!(
            decode_config(Some(&value)),
            Some(json!({ "expression": "0 9 * * *" }))
        );
    }

    #[test]
    fn unreadable_configs_decode_to_nothing_rather_than_panicking() {
        assert_eq!(decode_config(None), None);
        assert_eq!(decode_config(Some(&json!({}))), None);
        assert_eq!(
            decode_config(Some(&json!({ "base64": "not base64!" }))),
            None
        );
        assert_eq!(
            decode_config(Some(
                &json!({ "base64": base64::engine::general_purpose::STANDARD.encode("not json") })
            )),
            None
        );
    }

    #[test]
    fn every_recorded_alias_projects_onto_one_spelling() {
        assert_eq!(
            project_schedule(&json!({ "cron_expression": "0 9 * * *", "tz": "Europe/Berlin" })),
            ScheduleConfig {
                expression: Some("0 9 * * *".into()),
                timezone: Some("Europe/Berlin".into()),
                scheduled_for: None,
                last_fired: None,
            }
        );
        assert_eq!(
            project_schedule(&json!({ "schedule": "*/5 * * * *" })).expression,
            Some("*/5 * * * *".into())
        );
    }

    #[test]
    fn transport_secrets_never_reach_the_projection() {
        let projected = project_schedule(&json!({
            "expression": "0 9 * * *",
            "auth_token": "super-secret",
            "path": "/hook",
        }));
        let encoded = serde_json::to_string(&projected).expect("serializable");
        assert!(!encoded.contains("super-secret"));
        assert!(!encoded.contains("/hook"));
    }

    #[test]
    fn a_one_shot_schedule_needs_both_halves_of_its_local_time() {
        assert_eq!(
            project_schedule(
                &json!({ "scheduled_for": { "date": "2026-09-11", "time": "09:00" } })
            )
            .scheduled_for,
            Some(json!({ "date": "2026-09-11", "time": "09:00" }))
        );
        assert_eq!(
            project_schedule(&json!({ "scheduled_for": { "date": "2026-09-11" } })).scheduled_for,
            None
        );
    }
}
