//! Sink trigger utilities and HTTP endpoint
//!
//! Provides:
//! - `trigger_event` - Utility function for programmatic event triggering (Lambda, SQS, etc.)
//! - `http_trigger` - HTTP endpoint for HTTP sinks

use crate::{
    entity::{event_sink, execution_run},
    error::ApiError,
    execution::{
        DispatchRequest, ExecutionJwtParams, TokenType, is_jwt_configured, proxy_sse_response,
        sign_execution_jwt,
    },
    routes::app::events::db::get_event_from_db,
    state::AppState,
};
use axum::{
    Json,
    body::Body,
    extract::{Path, State},
    http::{HeaderMap, Method, StatusCode},
    response::{IntoResponse, Response},
};
use flow_like_types::{Result as FlResult, anyhow, create_id, tokio};
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Input for programmatic event triggering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerEventInput {
    /// The event ID to trigger
    pub event_id: String,
    /// Optional payload to pass to the event
    pub payload: Option<serde_json::Value>,
    /// User ID for attribution (optional)
    pub user_id: Option<String>,
}

/// Response from trigger operations
#[derive(Debug, Clone, Serialize)]
pub struct TriggerResponse {
    pub triggered: bool,
    pub run_id: Option<String>,
    pub message: String,
}

/// Utility function to trigger an event programmatically.
///
/// Use this in Lambda handlers, SQS processors, cron job workers, etc.
///
/// # Example
/// ```ignore
/// // In a Lambda handler
/// let result = trigger_event(&state, TriggerEventInput {
///     event_id: "event_123".to_string(),
///     payload: Some(json!({"key": "value"})),
///     user_id: Some("cron_worker".to_string()),
/// }).await?;
/// ```
pub async fn trigger_event(
    state: &AppState,
    input: TriggerEventInput,
) -> FlResult<TriggerResponse> {
    // Look up sink by event_id
    let sink = event_sink::Entity::find()
        .filter(event_sink::Column::EventId.eq(&input.event_id))
        .filter(event_sink::Column::Active.eq(true))
        .one(&state.db)
        .await?
        .ok_or_else(|| anyhow!("No active sink found for event {}", input.event_id))?;

    // Get the event from database
    let event = get_event_from_db(&state.db, &sink.event_id).await?;

    // Check JWT is configured
    if !is_jwt_configured() {
        return Err(anyhow!("Execution JWT signing not configured"));
    }

    // Create run
    let run_id = create_id();
    let expires_at = chrono::Utc::now().naive_utc() + chrono::Duration::hours(24);
    let user_id = input.user_id.unwrap_or_else(|| "system".to_string());

    let input_payload_len = input
        .payload
        .as_ref()
        .map(|p| {
            serde_json::to_string(p)
                .map(|s| s.len() as i64)
                .unwrap_or(0)
        })
        .unwrap_or(0);

    let event_json = serde_json::to_string(&event)?;

    // Get credentials
    let credentials = state.master_credentials().await?;
    let shared_credentials = credentials.into_shared_credentials();
    let credentials_json = serde_json::to_string(&shared_credentials)?;

    let callback_url =
        std::env::var("API_BASE_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());

    // Sign JWT
    let executor_jwt = sign_execution_jwt(ExecutionJwtParams {
        user_id: user_id.clone(),
        run_id: run_id.clone(),
        app_id: sink.app_id.clone(),
        board_id: event.board_id.clone(),
        event_id: Some(sink.event_id.clone()),
        callback_url: callback_url.clone(),
        token_type: TokenType::Executor,
        ttl_seconds: Some(24 * 60 * 60),
    })?;

    // Build dispatch request
    let request = DispatchRequest {
        run_id: run_id.clone(),
        app_id: sink.app_id.clone(),
        board_id: event.board_id.clone(),
        board_version: event.board_version,
        node_id: event.node_id.clone(),
        event_json: Some(event_json),
        payload: input.payload,
        user_id: user_id.clone(),
        credentials_json,
        jwt: executor_jwt,
        callback_url,
        token: None, // No PAT for system triggers
        oauth_tokens: None,
        stream_state: false,
        runtime_variables: None,
    };

    // Create run record
    let run = execution_run::ActiveModel {
        id: Set(run_id.clone()),
        board_id: Set(event.board_id.clone()),
        version: Set(None),
        event_id: Set(Some(sink.event_id.clone())),
        node_id: Set(Some(event.id.clone())),
        status: Set(execution_run::RunStatus::Pending),
        mode: Set(execution_run::RunMode::Http),
        log_level: Set(0),
        input_payload_len: Set(input_payload_len),
        input_payload_key: Set(None),
        output_payload_len: Set(0),
        error_message: Set(None),
        progress: Set(0),
        current_step: Set(None),
        started_at: Set(None),
        completed_at: Set(None),
        expires_at: Set(Some(expires_at)),
        user_id: Set(Some(user_id)),
        app_id: Set(sink.app_id.clone()),
        created_at: Set(chrono::Utc::now().naive_utc()),
        updated_at: Set(chrono::Utc::now().naive_utc()),
    };

    // Insert run record
    run.insert(&state.db).await?;

    // Dispatch (fire and forget for programmatic triggers)
    let dispatch_result = state.dispatcher.dispatch_http_sse(request).await;

    match dispatch_result {
        Ok(_) => Ok(TriggerResponse {
            triggered: true,
            run_id: Some(run_id),
            message: "Event triggered successfully".to_string(),
        }),
        Err(e) => Ok(TriggerResponse {
            triggered: false,
            run_id: Some(run_id),
            message: format!("Dispatch failed: {}", e),
        }),
    }
}

/// POST/GET/etc /sink/trigger/{app_id}/{path}
/// HTTP endpoint for HTTP sinks
#[tracing::instrument(name = "ANY /sink/trigger/{app_id}/{path}", skip(state, headers, body))]
pub async fn http_trigger(
    State(state): State<AppState>,
    Path((app_id, path)): Path<(String, String)>,
    method: Method,
    headers: HeaderMap,
    body: Body,
) -> Result<Response, ApiError> {
    // Normalize path
    let normalized_path = if path.starts_with('/') {
        path
    } else {
        format!("/{}", path)
    };

    tracing::info!(
        "HTTP sink trigger: {} {} for app {}",
        method.as_str(),
        normalized_path,
        app_id
    );

    // Look up sink by app_id and path
    let sink = event_sink::Entity::find()
        .filter(event_sink::Column::AppId.eq(&app_id))
        .filter(event_sink::Column::Path.eq(&normalized_path))
        .filter(event_sink::Column::Active.eq(true))
        .one(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Database error: {}", e);
            ApiError::internal_error(anyhow!("Database error"))
        })?;

    let sink = match sink {
        Some(s) => s,
        None => {
            tracing::warn!(
                "No active HTTP sink found for {} in app {}",
                normalized_path,
                app_id
            );
            return Ok((
                StatusCode::NOT_FOUND,
                Json(TriggerResponse {
                    triggered: false,
                    run_id: None,
                    message: "Route not found".to_string(),
                }),
            )
                .into_response());
        }
    };

    // Check auth token if set
    if let Some(expected_token) = &sink.auth_token {
        let provided_token = headers
            .get("Authorization")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.trim_start_matches("Bearer ").to_string());

        match provided_token {
            Some(token) if &token == expected_token => {}
            _ => {
                return Ok((
                    StatusCode::UNAUTHORIZED,
                    Json(TriggerResponse {
                        triggered: false,
                        run_id: None,
                        message: "Invalid or missing auth token".to_string(),
                    }),
                )
                    .into_response());
            }
        }
    }

    // Parse body
    let body_bytes = axum::body::to_bytes(body, 10 * 1024 * 1024) // 10MB limit
        .await
        .map_err(|e| {
            tracing::error!("Failed to read body: {}", e);
            ApiError::bad_request("Failed to read request body")
        })?;

    let payload: Option<serde_json::Value> = if !body_bytes.is_empty() {
        match serde_json::from_slice(&body_bytes) {
            Ok(v) => Some(v),
            Err(_) => Some(serde_json::Value::String(
                String::from_utf8_lossy(&body_bytes).to_string(),
            )),
        }
    } else {
        None
    };

    // Get the event from database (config lives in Event)
    let event = get_event_from_db(&state.db, &sink.event_id)
        .await
        .map_err(|e| ApiError::internal_error(anyhow!("Failed to get event: {}", e)))?;

    // Check JWT configured
    if !is_jwt_configured() {
        return Err(ApiError::internal_error(anyhow!(
            "Execution JWT signing not configured"
        )));
    }

    // Create run
    let run_id = create_id();
    let expires_at = chrono::Utc::now().naive_utc() + chrono::Duration::hours(24);

    let input_payload_len = payload
        .as_ref()
        .map(|p| {
            serde_json::to_string(p)
                .map(|s| s.len() as i64)
                .unwrap_or(0)
        })
        .unwrap_or(0);

    let event_json = serde_json::to_string(&event)
        .map_err(|e| ApiError::internal_error(anyhow!("Failed to serialize event: {}", e)))?;

    // Get credentials
    let credentials = state
        .master_credentials()
        .await
        .map_err(|e| ApiError::internal_error(anyhow!("Failed to get credentials: {}", e)))?;

    let shared_credentials = credentials.into_shared_credentials();
    let credentials_json = serde_json::to_string(&shared_credentials)
        .map_err(|e| ApiError::internal_error(anyhow!("Failed to serialize credentials: {}", e)))?;

    let callback_url =
        std::env::var("API_BASE_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());

    // Sign JWT
    let executor_jwt = sign_execution_jwt(ExecutionJwtParams {
        user_id: "http_sink".to_string(),
        run_id: run_id.clone(),
        app_id: app_id.clone(),
        board_id: event.board_id.clone(),
        event_id: Some(sink.event_id.clone()),
        callback_url: callback_url.clone(),
        token_type: TokenType::Executor,
        ttl_seconds: Some(24 * 60 * 60),
    })
    .map_err(|e| ApiError::internal_error(anyhow!("Failed to sign JWT: {}", e)))?;

    // Build dispatch request
    let request = DispatchRequest {
        run_id: run_id.clone(),
        app_id: app_id.clone(),
        board_id: event.board_id.clone(),
        board_version: event.board_version,
        node_id: event.node_id.clone(),
        event_json: Some(event_json),
        payload,
        user_id: "http_sink".to_string(),
        credentials_json,
        jwt: executor_jwt,
        callback_url,
        token: None,
        oauth_tokens: None,
        stream_state: false,
        runtime_variables: None,
    };

    // Create run record
    let run = execution_run::ActiveModel {
        id: Set(run_id.clone()),
        board_id: Set(event.board_id.clone()),
        version: Set(None),
        event_id: Set(Some(sink.event_id.clone())),
        node_id: Set(Some(event.id.clone())),
        status: Set(execution_run::RunStatus::Pending),
        mode: Set(execution_run::RunMode::Http),
        log_level: Set(0),
        input_payload_len: Set(input_payload_len),
        input_payload_key: Set(None),
        output_payload_len: Set(0),
        error_message: Set(None),
        progress: Set(0),
        current_step: Set(None),
        started_at: Set(None),
        completed_at: Set(None),
        expires_at: Set(Some(expires_at)),
        user_id: Set(None),
        app_id: Set(app_id.clone()),
        created_at: Set(chrono::Utc::now().naive_utc()),
        updated_at: Set(chrono::Utc::now().naive_utc()),
    };

    tracing::info!(run_id = %run_id, "Dispatching HTTP sink");

    // Insert run and dispatch
    let db_clone = state.db.clone();
    let run_id_clone = run_id.clone();
    let db_insert_handle = tokio::spawn(async move {
        run.insert(&db_clone).await.map_err(|e| {
            tracing::error!(error = %e, "Failed to create run record");
            e
        })
    });

    let dispatch_result = state.dispatcher.dispatch_http_sse(request).await;

    if let Err(e) = db_insert_handle.await {
        tracing::error!(run_id = %run_id_clone, error = ?e, "DB insert task failed");
    }

    match dispatch_result {
        Ok((_dispatch_response, executor_response)) => {
            tracing::info!(run_id = %run_id, "Got executor response, starting stream");
            Ok(
                proxy_sse_response(executor_response, run_id, Some(Arc::new(state.db.clone())))
                    .into_response(),
            )
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to dispatch");
            Ok((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(TriggerResponse {
                    triggered: false,
                    run_id: Some(run_id),
                    message: format!("Dispatch failed: {}", e),
                }),
            )
                .into_response())
        }
    }
}
