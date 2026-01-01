//! SQS message execution handler
//!
//! Parses SQS messages containing execution requests and runs them
//! using the flow-like executor with callback-based progress reporting.

use flow_like::credentials::SharedCredentials;
use flow_like_executor::{ExecutionRequest, ExecutorConfig};
use lambda_runtime::tracing::{self, instrument};
use serde::Deserialize;

/// Payload structure from the dispatcher (matches build_executor_payload)
#[derive(Debug, Deserialize)]
struct DispatchPayload {
    job_id: String,
    run_id: String,
    app_id: String,
    board_id: String,
    board_version: Option<(u32, u32, u32)>,
    node_id: String,
    event_json: Option<String>,
    payload: Option<serde_json::Value>,
    #[allow(dead_code)]
    user_id: String,
    credentials: serde_json::Value,
    executor_jwt: String,
    #[allow(dead_code)]
    callback_url: String,
    token: Option<String>,
    oauth_tokens: Option<std::collections::HashMap<String, flow_like_executor::OAuthTokenInput>>,
    #[serde(default)]
    stream_state: bool,
}

#[instrument(skip(body), fields(job_id, run_id, app_id))]
pub async fn execute(body: &str) -> flow_like_types::Result<()> {
    let payload: DispatchPayload = serde_json::from_str(body)
        .map_err(|e| flow_like_types::anyhow!("Failed to parse message: {}", e))?;

    tracing::Span::current()
        .record("job_id", &payload.job_id)
        .record("run_id", &payload.run_id)
        .record("app_id", &payload.app_id);

    tracing::info!(
        job_id = %payload.job_id,
        run_id = %payload.run_id,
        app_id = %payload.app_id,
        board_id = %payload.board_id,
        "Processing execution request"
    );

    let credentials: SharedCredentials = serde_json::from_value(payload.credentials)
        .map_err(|e| flow_like_types::anyhow!("Failed to parse credentials: {}", e))?;

    let request = ExecutionRequest {
        credentials,
        app_id: payload.app_id,
        board_id: payload.board_id,
        board_version: payload.board_version,
        node_id: payload.node_id,
        event_json: payload.event_json,
        payload: payload.payload,
        executor_jwt: payload.executor_jwt,
        token: payload.token,
        oauth_tokens: payload.oauth_tokens,
        stream_state: payload.stream_state,
    };

    let config = ExecutorConfig::from_env();

    match flow_like_executor::execute(request, config).await {
        Ok(result) => {
            tracing::info!(
                run_id = %result.run_id,
                status = ?result.status,
                duration_ms = result.duration_ms,
                "Execution completed"
            );
            Ok(())
        }
        Err(e) => {
            tracing::error!(error = %e, "Execution failed");
            Err(flow_like_types::anyhow!("Execution failed: {}", e))
        }
    }
}
