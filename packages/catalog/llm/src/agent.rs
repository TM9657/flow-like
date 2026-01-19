use flow_like::bit::Bit;
use flow_like_model_provider::history::{History, Tool};
use flow_like_types::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

pub mod from_model;
pub mod helpers;
pub mod invoke;
pub mod register_mcp_tools;
pub mod register_thinking;
pub mod register_tools;
pub mod set_system_prompt;
pub mod simple;
pub mod stream_invoke;

/// MCP server registration with optional tool filtering
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct McpServerConfig {
    /// URI of the MCP server
    pub uri: String,

    /// Optional tool filter - if None, all tools are used
    /// If Some, only tools in this set are used
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_filter: Option<HashSet<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Agent {
    /// The LLM model id backing this agent
    pub model: Bit,

    /// Model display name
    pub model_display_name: Option<String>,

    /// Maximum number of iterations/tool calls before stopping
    pub max_iterations: u64,

    /// System prompt for the agent
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,

    /// Registered tools (function calling schemas for non-function tools)
    #[serde(default)]
    pub tools: Vec<Tool>,

    /// Function references (node_id -> node_name mapping)
    /// These are converted to tools at execution time to keep data slim
    #[serde(default)]
    pub function_refs: HashMap<String, String>,

    /// MCP servers with optional tool filtering
    #[serde(default)]
    pub mcp_servers: Vec<McpServerConfig>,

    /// Whether the thinking tool is enabled
    #[serde(default)]
    pub thinking_enabled: bool,

    /// Optional conversation history to initialize with
    #[serde(skip_serializing_if = "Option::is_none")]
    pub history: Option<History>,
}

impl Agent {
    /// Create a new agent from model id with default configuration
    pub fn new(model: Bit, max_iterations: u64) -> Self {
        Self {
            model,
            model_display_name: None,
            max_iterations,
            system_prompt: None,
            tools: Vec::new(),
            function_refs: HashMap::new(),
            mcp_servers: Vec::new(),
            thinking_enabled: false,
            history: None,
        }
    }

    /// Add a tool to this agent (for non-function tools)
    pub fn add_tool(&mut self, tool: Tool) {
        self.tools.push(tool);
    }

    /// Add a function reference (node_id -> node_name)
    pub fn add_function_ref(&mut self, node_id: String, node_name: String) {
        self.function_refs.insert(node_id, node_name);
    }

    /// Add an MCP server configuration
    pub fn add_mcp_server(&mut self, config: McpServerConfig) {
        self.mcp_servers.push(config);
    }

    /// Enable the thinking tool
    pub fn enable_thinking(&mut self) {
        self.thinking_enabled = true;
    }

    /// Set the system prompt
    pub fn set_system_prompt(&mut self, prompt: String) {
        self.system_prompt = Some(prompt);
    }

    /// Set conversation history
    pub fn set_history(&mut self, history: History) {
        self.history = Some(history);
    }

    /// Get the effective system prompt (from history or direct field)
    pub fn get_system_prompt(&self) -> Option<String> {
        if let Some(prompt) = &self.system_prompt {
            return Some(prompt.clone());
        }
        if let Some(history) = &self.history {
            return history.get_system_prompt();
        }
        None
    }
}
