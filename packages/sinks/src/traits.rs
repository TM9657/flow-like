//! Sink trait definitions

use crate::types::{SinkRegistration, SinkType};
use std::sync::Arc;

/// Result type for sink operations
pub type SinkResult<T> = Result<T, SinkError>;

/// Error type for sink operations
#[derive(Debug, thiserror::Error)]
pub enum SinkError {
    #[error("Sink not found: {0}")]
    NotFound(String),

    #[error("Sink already exists: {0}")]
    AlreadyExists(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("Authentication failed: {0}")]
    AuthFailed(String),

    #[error("Execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

/// Response from triggering a sink
#[derive(Debug, Clone)]
pub struct TriggerResponse {
    /// Whether the event was successfully triggered
    pub triggered: bool,

    /// The run ID if execution was started
    pub run_id: Option<String>,

    /// Immediate response data (for sync triggers)
    pub response: Option<flow_like_types::Value>,

    /// Error message if trigger failed
    pub error: Option<String>,
}

impl TriggerResponse {
    pub fn success(run_id: Option<String>) -> Self {
        Self {
            triggered: true,
            run_id,
            response: None,
            error: None,
        }
    }

    pub fn with_response(run_id: Option<String>, response: flow_like_types::Value) -> Self {
        Self {
            triggered: true,
            run_id,
            response: Some(response),
            error: None,
        }
    }

    pub fn failed(error: impl Into<String>) -> Self {
        Self {
            triggered: false,
            run_id: None,
            response: None,
            error: Some(error.into()),
        }
    }
}

/// Context for sink operations
pub struct SinkContext<E: Executor> {
    /// The executor for running flows
    pub executor: Arc<E>,
}

/// Executor trait that sinks use to trigger flow execution
#[async_trait::async_trait]
pub trait Executor: Send + Sync {
    /// Execute a flow event
    async fn execute_event(
        &self,
        app_id: &str,
        board_id: &str,
        event_id: &str,
        payload: Option<flow_like_types::Value>,
        personal_access_token: Option<&str>,
    ) -> SinkResult<String>;
}

/// Trait for sink implementations
#[async_trait::async_trait]
pub trait SinkTrait: Send + Sync {
    /// Get the sink type
    fn sink_type(&self) -> SinkType;

    /// Check if this sink can run on server
    fn is_server_compatible(&self) -> bool {
        self.sink_type().is_server_available()
    }

    /// Check if this sink can run on desktop
    fn is_desktop_compatible(&self) -> bool {
        self.sink_type().is_desktop_available()
    }

    /// Validate sink configuration
    fn validate_config(&self, config: &flow_like_types::Value) -> SinkResult<()>;

    /// Register a new event with this sink
    async fn register<E: Executor>(
        &self,
        ctx: &SinkContext<E>,
        registration: &SinkRegistration,
    ) -> SinkResult<()>;

    /// Unregister an event from this sink
    async fn unregister<E: Executor>(
        &self,
        ctx: &SinkContext<E>,
        registration: &SinkRegistration,
    ) -> SinkResult<()>;

    /// Handle an incoming event trigger
    async fn handle_trigger<E: Executor>(
        &self,
        ctx: &SinkContext<E>,
        registration: &SinkRegistration,
        payload: Option<flow_like_types::Value>,
    ) -> SinkResult<TriggerResponse>;
}
