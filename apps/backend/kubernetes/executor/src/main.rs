//! Kubernetes Executor for Flow-Like
//!
//! This executor runs in Kubernetes and supports both:
//! - Server mode (EXECUTOR_SERVER_MODE=true): Runs as a long-lived HTTP server
//! - Job mode (default): Runs a single job and exits
//!
//! ## Endpoints (server mode)
//!
//! - `POST /execute` - Execute with callback (async)
//! - `POST /execute/stream` - Execute with NDJSON streaming
//! - `POST /execute/sse` - Execute with Server-Sent Events
//! - `GET /health` - Health check
//! - `GET /metrics` - Prometheus metrics

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use axum::routing::get;
use flow_like_executor::{ExecutorState, executor_router};

mod metrics;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    metrics::init_telemetry();

    let server_mode = std::env::var("EXECUTOR_SERVER_MODE")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    if server_mode {
        run_server().await
    } else {
        run_job_once().await
    }
}

async fn run_server() -> Result<(), Box<dyn std::error::Error>> {
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let metrics_port = std::env::var("METRICS_PORT").unwrap_or_else(|_| "9090".to_string());

    // Create executor state from environment
    let state = ExecutorState::from_env();

    // Use the standard executor router with all endpoints
    let app = executor_router(state).route("/metrics", get(metrics::handler));

    let metrics_app = axum::Router::new().route("/metrics", get(metrics::handler));

    let addr = format!("0.0.0.0:{}", port);
    let metrics_addr = format!("0.0.0.0:{}", metrics_port);

    tracing::info!(%addr, "Executor pool server listening");
    tracing::info!(%metrics_addr, "Metrics server listening");

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    let metrics_listener = tokio::net::TcpListener::bind(&metrics_addr).await?;

    tokio::select! {
        res = axum::serve(listener, app) => res?,
        res = axum::serve(metrics_listener, metrics_app) => res?,
    }
    Ok(())
}

async fn run_job_once() -> Result<(), Box<dyn std::error::Error>> {
    // Job-once mode is for K8s Jobs that run a single execution and exit.
    // This is a placeholder - the actual implementation would need to:
    // 1. Fetch job input from a pre-signed URL or ConfigMap
    // 2. Build an ExecutionRequest with proper credentials
    // 3. Execute the flow
    // 4. Upload results to a pre-signed URL
    //
    // For now, the executor pool (server mode) handles all executions.
    // Job-once mode can be implemented when needed for batch/scheduled workloads.

    tracing::error!(
        "Job-once mode is not yet implemented. Use EXECUTOR_SERVER_MODE=true for the executor pool."
    );
    std::process::exit(1);
}
