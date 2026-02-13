//! Axum routes for the executor
//!
//! Provides both callback-based and streaming execution endpoints.

use crate::config::ExecutorConfig;
use crate::execute::execute;
use crate::streaming::{event_to_ndjson, execute_streaming};
use crate::types::ExecutionRequest;
use axum::body::Body;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{Response, Sse};
use axum::routing::{get, post};
use axum::{Json, Router};
use futures_util::stream::StreamExt;
use serde::Serialize;
use std::convert::Infallible;
use std::sync::Arc;

/// Shared executor state
#[derive(Clone)]
pub struct ExecutorState {
    pub config: ExecutorConfig,
}

impl ExecutorState {
    pub fn new(config: ExecutorConfig) -> Self {
        Self { config }
    }

    pub fn from_env() -> Self {
        Self::new(ExecutorConfig::from_env())
    }
}

/// Health check response
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub service: String,
    pub version: String,
}

/// Construct the executor router with all endpoints
pub fn executor_router(state: ExecutorState) -> Router {
    Router::new()
        .route("/execute", post(execute_callback))
        .route("/execute/stream", post(execute_stream))
        .route("/execute/sse", post(execute_sse))
        .route("/health", get(health_check))
        .with_state(Arc::new(state))
}

/// Health check endpoint
async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".to_string(),
        service: "flow-like-executor".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// Execute with callback-based progress reporting
///
/// POST /execute
///
/// Events are sent to the callback URL specified in the JWT.
/// Returns immediately with status and waits for completion.
#[derive(Debug, Serialize)]
pub struct ExecuteResponse {
    pub run_id: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub duration_ms: u64,
}

async fn execute_callback(
    State(state): State<Arc<ExecutorState>>,
    Json(request): Json<ExecutionRequest>,
) -> Result<Json<ExecuteResponse>, (StatusCode, String)> {
    match execute(request, state.config.clone()).await {
        Ok(result) => Ok(Json(ExecuteResponse {
            run_id: result.run_id,
            status: format!("{:?}", result.status).to_lowercase(),
            error: result.error,
            duration_ms: result.duration_ms,
        })),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

/// Execute with streaming response (newline-delimited JSON)
///
/// POST /execute/stream
///
/// Returns a streaming response with events as NDJSON.
/// Each line is a complete JSON object.
async fn execute_stream(
    State(state): State<Arc<ExecutorState>>,
    Json(request): Json<ExecutionRequest>,
) -> Result<Response, (StatusCode, String)> {
    let stream = execute_streaming(request, state.config.clone())
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let body_stream = stream.map(|event| Ok::<_, Infallible>(event_to_ndjson(&event)));

    let body = Body::from_stream(body_stream);

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/x-ndjson")
        .header("Transfer-Encoding", "chunked")
        .header("Cache-Control", "no-cache")
        .body(body)
        .unwrap())
}

/// Execute with Server-Sent Events
///
/// POST /execute/sse
///
/// Returns a streaming response using SSE format.
async fn execute_sse(
    State(state): State<Arc<ExecutorState>>,
    Json(request): Json<ExecutionRequest>,
) -> Result<
    Sse<impl futures_util::Stream<Item = Result<axum::response::sse::Event, Infallible>>>,
    (StatusCode, String),
> {
    let stream = execute_streaming(request, state.config.clone())
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let sse_stream = stream.map(|event| {
        let data = serde_json::to_string(&event).unwrap_or_default();
        // All events are now InterComEvent, use event_type for SSE event field
        let event_type = &event.event_type;
        Ok::<_, Infallible>(
            axum::response::sse::Event::default()
                .event(event_type)
                .data(data),
        )
    });

    Ok(Sse::new(sse_stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(std::time::Duration::from_secs(15))
            .text("ping"),
    ))
}
