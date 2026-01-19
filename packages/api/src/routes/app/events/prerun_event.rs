//! Pre-run analysis endpoint for events
//!
//! Returns information needed before executing an event:
//! - Runtime-configured variables that need values
//! - Required OAuth providers and scopes

use crate::{
    ensure_permission, error::ApiError, middleware::jwt::AppUser,
    permission::role_permission::RolePermissions, state::AppState,
};
use axum::{
    Extension, Json,
    extract::{Path, Query, State},
};
use flow_like::flow::board::ExecutionMode;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Query parameters for pre-run analysis
#[derive(Debug, Deserialize)]
pub struct PrerunEventQuery {
    /// Board version as tuple (major, minor, patch) - defaults to latest
    pub version: Option<String>,
}

/// A runtime-configured variable that needs a value before execution
#[derive(Debug, Serialize)]
pub struct RuntimeVariable {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub data_type: String,
    pub value_type: String,
    pub secret: bool,
    pub schema: Option<String>,
}

/// OAuth provider requirement
#[derive(Debug, Serialize)]
pub struct OAuthRequirement {
    pub provider_id: String,
    pub scopes: Vec<String>,
}

/// Response from pre-run analysis
#[derive(Debug, Serialize)]
pub struct PrerunEventResponse {
    /// ID of the board this event triggers
    pub board_id: String,
    /// Variables that are marked as runtime_configured (need user-provided values)
    pub runtime_variables: Vec<RuntimeVariable>,
    /// OAuth providers required by nodes in this board
    pub oauth_requirements: Vec<OAuthRequirement>,
    /// Whether the event can only run locally (has offline-only nodes)
    pub requires_local_execution: bool,
    /// Board's execution mode setting (Hybrid, Remote, Local)
    pub execution_mode: ExecutionMode,
    /// Whether the user can execute locally (has ReadBoards permission)
    /// If false, execution must happen on server
    pub can_execute_locally: bool,
}

fn parse_version(version_str: &str) -> Option<(u32, u32, u32)> {
    let parts: Vec<&str> = version_str.split('_').collect();
    if parts.len() == 3 {
        let major = parts[0].parse().ok()?;
        let minor = parts[1].parse().ok()?;
        let patch = parts[2].parse().ok()?;
        Some((major, minor, patch))
    } else {
        None
    }
}

/// Analyze an event to determine what's needed before execution.
///
/// Returns runtime-configured variables and OAuth requirements from the event's board.
#[tracing::instrument(
    name = "GET /apps/{app_id}/events/{event_id}/prerun",
    skip(state, user)
)]
pub async fn prerun_event(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Path((app_id, event_id)): Path<(String, String)>,
    Query(query): Query<PrerunEventQuery>,
) -> Result<Json<PrerunEventResponse>, ApiError> {
    let permission = ensure_permission!(user, &app_id, &state, RolePermissions::ExecuteEvents);
    let sub = permission.sub()?;

    // Check if user can execute locally (has ReadBoards permission)
    let can_execute_locally = permission.has_permission(RolePermissions::ReadBoards);

    let version = query.version.as_ref().and_then(|v| parse_version(v));

    // Get the event to find the associated board
    let app = state.master_app(&sub, &app_id, &state).await?;
    let event = app.get_event(&event_id, None).await?;
    let board_id = event.board_id.clone();

    // Get the board from the event
    let board = state
        .master_board(&sub, &app_id, &board_id, &state, version)
        .await?;

    // Collect runtime-configured variables
    let runtime_variables: Vec<RuntimeVariable> = board
        .variables
        .values()
        .filter(|v| v.runtime_configured)
        .map(|v| RuntimeVariable {
            id: v.id.clone(),
            name: v.name.clone(),
            description: v.description.clone(),
            data_type: format!("{:?}", v.data_type),
            value_type: format!("{:?}", v.value_type),
            secret: v.secret,
            schema: v.schema.clone(),
        })
        .collect();

    // Collect OAuth requirements from all nodes (including layers)
    let mut oauth_scopes: HashMap<String, Vec<String>> = HashMap::new();
    let mut requires_local_execution = false;

    let process_node = |node: &flow_like::flow::node::Node,
                        oauth_scopes: &mut HashMap<String, Vec<String>>,
                        requires_local: &mut bool| {
        // Check if node requires local execution
        if node.only_offline {
            *requires_local = true;
        }

        // Collect OAuth provider IDs
        if let Some(providers) = &node.oauth_providers {
            for provider_id in providers {
                oauth_scopes.entry(provider_id.clone()).or_default();
            }
        }

        // Collect required scopes
        if let Some(required_scopes) = &node.required_oauth_scopes {
            for (provider_id, scopes) in required_scopes {
                let entry = oauth_scopes.entry(provider_id.clone()).or_default();
                for scope in scopes {
                    if !entry.contains(scope) {
                        entry.push(scope.clone());
                    }
                }
            }
        }
    };

    // Process main board nodes
    for node in board.nodes.values() {
        process_node(node, &mut oauth_scopes, &mut requires_local_execution);
    }

    // Process layer nodes
    for layer in board.layers.values() {
        for node in layer.nodes.values() {
            process_node(node, &mut oauth_scopes, &mut requires_local_execution);
        }
    }

    let oauth_requirements: Vec<OAuthRequirement> = oauth_scopes
        .into_iter()
        .map(|(provider_id, scopes)| OAuthRequirement {
            provider_id,
            scopes,
        })
        .collect();

    Ok(Json(PrerunEventResponse {
        board_id,
        runtime_variables,
        oauth_requirements,
        requires_local_execution,
        execution_mode: board.execution_mode.clone(),
        can_execute_locally,
    }))
}
