//! GitHub Copilot integration nodes
//!
//! This module provides visual nodes for interacting with GitHub Copilot's AI capabilities
//! through the copilot-sdk-rust crate.

pub mod client;
pub mod config;
pub mod invoke;
pub mod mcp;
pub mod session;
pub mod tools;
pub mod utilities;

#[cfg(feature = "execute")]
use flow_like_types::Cacheable;
use flow_like_types::JsonSchema;
use serde::{Deserialize, Serialize};
#[cfg(feature = "execute")]
use std::any::Any;
use std::collections::HashMap;
#[cfg(feature = "execute")]
use std::sync::Arc;

/// Handle reference to a running Copilot client stored in execution cache
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CopilotClientHandle {
    /// Unique cache key for this client instance
    pub cache_key: String,
}

/// Handle reference to a Copilot session stored in execution cache
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CopilotSessionHandle {
    /// Unique cache key for this session instance
    pub cache_key: String,
    /// Session ID from Copilot
    pub session_id: String,
    /// Reference to parent client
    pub client_key: String,
}

/// Cached Copilot client wrapper
#[cfg(feature = "execute")]
pub struct CachedCopilotClient {
    pub client: copilot_sdk::Client,
}

#[cfg(feature = "execute")]
impl Cacheable for CachedCopilotClient {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Cached Copilot session wrapper
#[cfg(feature = "execute")]
pub struct CachedCopilotSession {
    pub session: Arc<copilot_sdk::Session>,
}

#[cfg(feature = "execute")]
impl Cacheable for CachedCopilotSession {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

fn default_true() -> bool {
    true
}

/// Log level for Copilot client
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "lowercase")]
pub enum CopilotLogLevel {
    #[default]
    Error,
    Warn,
    Info,
    Debug,
}

/// Session configuration for creating Copilot sessions
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct CopilotSessionConfig {
    /// Model ID override
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Enable streaming responses
    #[serde(default = "default_true")]
    pub streaming: bool,
    /// Request permission for actions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_permission: Option<bool>,
    /// System message configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_message: Option<SystemMessageConfig>,
    /// Infinite session configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub infinite_sessions: Option<InfiniteSessionConfig>,
    /// MCP servers configuration
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub mcp_servers: HashMap<String, flow_like_types::Value>,
    /// Custom agents configuration
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub custom_agents: Vec<CustomAgentConfig>,
    /// Tools configuration
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<CopilotToolConfig>,
    /// Provider configuration (BYOK)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<ProviderConfig>,
}

/// System message configuration
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SystemMessageConfig {
    /// System message content
    pub content: String,
    /// How to apply the message: Replace or Append
    #[serde(default)]
    pub mode: SystemMessageMode,
}

/// Mode for applying system message
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "lowercase")]
pub enum SystemMessageMode {
    #[default]
    Replace,
    Append,
}

/// Infinite session configuration for automatic context compaction
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct InfiniteSessionConfig {
    /// Enable infinite sessions
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Background compaction threshold (0.0 to 1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background_compaction_threshold: Option<f64>,
    /// Buffer exhaustion threshold (0.0 to 1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub buffer_exhaustion_threshold: Option<f64>,
}

impl Default for InfiniteSessionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            background_compaction_threshold: None,
            buffer_exhaustion_threshold: None,
        }
    }
}

/// Custom agent configuration
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CustomAgentConfig {
    /// Agent identifier
    pub name: String,
    /// Display name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    /// Description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Agent system prompt
    pub prompt: String,
    /// Model override for this agent
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// MCP servers for this agent
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub mcp_servers: HashMap<String, flow_like_types::Value>,
}

/// Tool configuration for Copilot
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CopilotToolConfig {
    /// Tool name
    pub name: String,
    /// Tool description
    pub description: String,
    /// JSON schema for tool parameters
    pub schema: flow_like_types::Value,
}

/// Local MCP server configuration
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct McpLocalServerConfig {
    /// Server type (always "local" for this config)
    #[serde(rename = "type", default = "default_local_type")]
    pub server_type: String,
    /// Command to execute
    pub command: String,
    /// Command arguments
    #[serde(default)]
    pub args: Vec<String>,
    /// Environment variables
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub env: HashMap<String, String>,
    /// Working directory
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    /// Tool filter (use ["*"] for all tools)
    #[serde(default = "default_tool_filter")]
    pub tools: Vec<String>,
    /// Server timeout in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<i32>,
}

fn default_local_type() -> String {
    "local".to_string()
}

fn default_tool_filter() -> Vec<String> {
    vec!["*".to_string()]
}

/// HTTP/SSE MCP server configuration
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct McpHttpServerConfig {
    /// Server type (always "http" for this config)
    #[serde(rename = "type", default = "default_http_type")]
    pub server_type: String,
    /// HTTP endpoint URL
    pub url: String,
    /// Custom headers
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub headers: HashMap<String, String>,
    /// Tool filter (use ["*"] for all tools)
    #[serde(default = "default_tool_filter")]
    pub tools: Vec<String>,
    /// Server timeout in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<i32>,
}

fn default_http_type() -> String {
    "http".to_string()
}

/// Provider configuration for BYOK (Bring Your Own Key)
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ProviderConfig {
    /// Provider API base URL
    pub base_url: String,
    /// API key (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    /// Model override
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

/// Model information returned by list_models
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CopilotModelInfo {
    /// Model ID
    pub id: String,
    /// Model display name
    pub name: String,
}

/// Status information from get_status
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CopilotStatusInfo {
    /// CLI version
    pub version: String,
    /// Protocol version
    pub protocol_version: String,
}

/// Authentication status from get_auth_status
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CopilotAuthStatus {
    /// Whether the user is authenticated
    pub authenticated: bool,
    /// GitHub username if authenticated
    #[serde(skip_serializing_if = "Option::is_none")]
    pub login: Option<String>,
}

/// Constants for cache key prefixes
pub const COPILOT_CLIENT_PREFIX: &str = "copilot_client_";
pub const COPILOT_SESSION_PREFIX: &str = "copilot_session_";
