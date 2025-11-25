use std::sync::Arc;

use async_trait::async_trait;
use flow_like_types::Result;
use rig::{
    client::completion::CompletionClientDyn, completion::ToolDefinition, tool::Tool,
    tools::ThinkTool,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::app::App;
use crate::bit::{Bit, BitModelPreference, BitTypes, LLMParameters, Metadata};
use crate::flow::board::Board;
use crate::flow::node::Node;
use crate::flow::pin::PinType;
use crate::profile::Profile;
use crate::state::FlowLikeState;
use flow_like_model_provider::provider::ModelProvider;
use flow_like_types::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeMetadata {
    pub name: String,
    pub friendly_name: String,
    pub description: String,
    pub inputs: Vec<PinMetadata>,
    pub outputs: Vec<PinMetadata>,
    pub category: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PinMetadata {
    pub name: String, // INTERNAL name - use this in commands! (e.g., "body_text")
    pub friendly_name: String, // Display name (e.g., "Body (text)")
    pub description: String, // Pin description
    pub data_type: String, // e.g., "String", "Integer", "Struct", "Generic", "Execution"
    pub value_type: String, // e.g., "Normal", "Array", "HashMap", "HashSet"
    pub schema: Option<String>, // JSON schema for Struct types
    pub is_generic: bool, // Generic pins can connect to any type
    pub valid_values: Option<Vec<String>>, // For enum-like pins
    pub enforce_schema: bool, // If true, schema must match exactly
}

#[async_trait]
pub trait CatalogProvider: Send + Sync {
    async fn search(&self, query: &str) -> Vec<NodeMetadata>;
    async fn search_by_pin_type(&self, pin_type: &str, is_input: bool) -> Vec<NodeMetadata>;
    async fn filter_by_category(&self, category_prefix: &str) -> Vec<NodeMetadata>;
    async fn get_all_nodes(&self) -> Vec<String>;
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Edge {
    pub id: String,
    pub from: String,
    pub from_pin: String,
    pub to: String,
    pub to_pin: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Suggestion {
    pub node_type: String,
    pub reason: String,
    pub connection_description: String,
    pub position: Option<NodePosition>,
    pub connections: Vec<Connection>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NodePosition {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Connection {
    pub from_node_id: String,
    pub from_pin: String,
    pub to_pin: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PlanStep {
    pub id: String,
    pub description: String,
    pub status: PlanStepStatus,
    pub tool_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum PlanStepStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum StreamEvent {
    Token(String),
    PlanStep(PlanStep),
    ToolCall {
        name: String,
        args: String,
    },
    ToolResult {
        name: String,
        result: String,
    },
    Thinking(String),
    FocusNode {
        node_id: String,
        description: String,
    },
}

/// Represents the type of response from the copilot
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AgentType {
    /// Response that primarily explains
    Explain,
    /// Response that includes modifications
    Edit,
}

/// A message in the chat history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
}

/// Role in the chat conversation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ChatRole {
    User,
    Assistant,
}

/// Response from the copilot that may include commands for the frontend to execute
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopilotResponse {
    pub agent_type: AgentType,
    pub message: String,
    pub commands: Vec<BoardCommand>,
    pub suggestions: Vec<Suggestion>,
}

/// A single command with its summary for the UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardCommandWithSummary {
    pub command: BoardCommand,
    pub summary: String,
}

/// Commands that can be executed on the board
/// These are sent to the frontend which executes them to maintain undo/redo history
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "command_type")]
pub enum BoardCommand {
    AddNode {
        node_type: String,
        ref_id: Option<String>,
        position: Option<NodePosition>,
        #[serde(default)]
        summary: Option<String>,
    },
    RemoveNode {
        node_id: String,
        #[serde(default)]
        summary: Option<String>,
    },
    ConnectPins {
        from_node: String,
        from_pin: String,
        to_node: String,
        to_pin: String,
        #[serde(default)]
        summary: Option<String>,
    },
    DisconnectPins {
        from_node: String,
        from_pin: String,
        to_node: String,
        to_pin: String,
        #[serde(default)]
        summary: Option<String>,
    },
    UpdateNodePin {
        node_id: String,
        pin_id: String,
        value: serde_json::Value,
        #[serde(default)]
        summary: Option<String>,
    },
    MoveNode {
        node_id: String,
        position: NodePosition,
        #[serde(default)]
        summary: Option<String>,
    },
    // Variable management
    CreateVariable {
        name: String,
        data_type: String,  // "String", "Integer", "Float", "Boolean", "Struct"
        value_type: String, // "Normal", "Array", "HashMap", "HashSet"
        default_value: Option<serde_json::Value>,
        description: Option<String>,
        #[serde(default)]
        summary: Option<String>,
    },
    RemoveVariable {
        variable_id: String,
        #[serde(default)]
        summary: Option<String>,
    },
    // Comment management
    AddComment {
        content: String,
        position: NodePosition,
        width: Option<f64>,
        height: Option<f64>,
        color: Option<String>,
        #[serde(default)]
        summary: Option<String>,
    },
    RemoveComment {
        comment_id: String,
        #[serde(default)]
        summary: Option<String>,
    },
    // Layer/grouping management
    CreateLayer {
        name: String,
        node_ids: Vec<String>, // Nodes to include in the layer
        position: Option<NodePosition>,
        color: Option<String>,
        #[serde(default)]
        summary: Option<String>,
    },
    RemoveLayer {
        layer_id: String,
        #[serde(default)]
        summary: Option<String>,
    },
}

#[derive(Debug, Serialize, Deserialize)]
struct GraphContext {
    nodes: Vec<NodeContext>,
    edges: Vec<EdgeContext>,
    selected_nodes: Vec<String>,
}

/// Compact node representation for context
#[derive(Debug, Serialize, Deserialize)]
struct NodeContext {
    id: String,
    #[serde(rename = "t")] // "type" abbreviated
    node_type: String,
    #[serde(rename = "n")] // "name" abbreviated
    friendly_name: String,
    #[serde(rename = "i")] // "inputs" abbreviated
    inputs: Vec<PinContext>,
    #[serde(rename = "o")] // "outputs" abbreviated
    outputs: Vec<PinContext>,
    #[serde(rename = "p")] // "position" abbreviated
    position: (i32, i32),
    #[serde(rename = "s")] // "size" abbreviated
    estimated_size: (u16, u16),
}

/// Compact pin representation
#[derive(Debug, Serialize, Deserialize)]
struct PinContext {
    #[serde(rename = "n")] // "name" abbreviated
    name: String,
    #[serde(rename = "t")] // "type" abbreviated
    type_name: String,
    /// Only included if pin has a non-empty default value
    #[serde(rename = "v", skip_serializing_if = "Option::is_none")] // "value" abbreviated
    default_value: Option<String>,
}

/// Compact edge representation
#[derive(Debug, Serialize, Deserialize)]
struct EdgeContext {
    #[serde(rename = "f")] // "from" abbreviated
    from_node_id: String,
    #[serde(rename = "fp")] // "from_pin" abbreviated
    from_pin_name: String,
    #[serde(rename = "t")] // "to" abbreviated
    to_node_id: String,
    #[serde(rename = "tp")] // "to_pin" abbreviated
    to_pin_name: String,
}

#[derive(Deserialize)]
struct SearchArgs {
    query: String,
}

#[derive(Deserialize)]
struct SearchByPinArgs {
    pin_type: String,
    is_input: bool,
}

#[derive(Deserialize)]
struct FilterCategoryArgs {
    category_prefix: String,
}

#[derive(Deserialize)]
struct SearchTemplatesArgs {
    query: String,
}

/// Compact template info for model context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateInfo {
    pub id: String,
    pub app_id: String,
    pub name: String,
    pub description: String,
    pub tags: Vec<String>,
    pub node_count: usize,
    pub node_types: Vec<String>,
}

#[derive(Deserialize)]
struct ThinkingArgs {
    thought: String,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct FocusNodeArgs {
    node_id: String,
    description: String,
}

#[derive(Deserialize)]
struct EmitCommandsArgs {
    commands: Vec<BoardCommand>,
    explanation: String,
}

#[derive(Debug, thiserror::Error)]
#[error("Catalog tool error")]
struct CatalogToolError;

#[derive(Debug, thiserror::Error)]
#[error("Template tool error")]
struct TemplateToolError;

#[derive(Debug, thiserror::Error)]
#[error("Focus node tool error")]
struct FocusNodeToolError;

#[derive(Debug, thiserror::Error)]
#[error("Emit commands tool error")]
struct EmitCommandsToolError;

struct CatalogTool {
    provider: Arc<dyn CatalogProvider>,
}

impl Tool for CatalogTool {
    const NAME: &'static str = "catalog_search";

    type Error = CatalogToolError;
    type Args = SearchArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "catalog_search".to_string(),
            description: "Search for nodes in the catalog by functionality or name".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search query"
                    }
                },
                "required": ["query"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let matches = self.provider.search(&args.query).await;
        Ok(serde_json::to_string(&matches).unwrap_or_default())
    }
}

struct SearchByPinTool {
    provider: Arc<dyn CatalogProvider>,
}

impl Tool for SearchByPinTool {
    const NAME: &'static str = "search_by_pin";

    type Error = CatalogToolError;
    type Args = SearchByPinArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "search_by_pin".to_string(),
            description: "Search for nodes that have a specific input or output pin type"
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "pin_type": {
                        "type": "string",
                        "description": "The type of pin to search for (e.g., 'String', 'Number')"
                    },
                    "is_input": {
                        "type": "boolean",
                        "description": "True to search input pins, false for output pins"
                    }
                },
                "required": ["pin_type", "is_input"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let matches = self
            .provider
            .search_by_pin_type(&args.pin_type, args.is_input)
            .await;
        Ok(serde_json::to_string(&matches).unwrap_or_default())
    }
}

struct FilterCategoryTool {
    provider: Arc<dyn CatalogProvider>,
}

impl Tool for FilterCategoryTool {
    const NAME: &'static str = "filter_category";

    type Error = CatalogToolError;
    type Args = FilterCategoryArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "filter_category".to_string(),
            description: "Filter nodes by category prefix".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "category_prefix": {
                        "type": "string",
                        "description": "The category prefix to filter by (e.g., 'math', 'logic')"
                    }
                },
                "required": ["category_prefix"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let matches = self
            .provider
            .filter_by_category(&args.category_prefix)
            .await;
        Ok(serde_json::to_string(&matches).unwrap_or_default())
    }
}

struct SearchTemplatesTool {
    templates: Vec<TemplateInfo>,
    current_template_id: Option<String>,
}

impl Tool for SearchTemplatesTool {
    const NAME: &'static str = "search_templates";

    type Error = TemplateToolError;
    type Args = SearchTemplatesArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "search_templates".to_string(),
            description: "Search for workflow templates that can serve as examples or starting points. Templates show real implementations of common patterns.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search query (matches name, description, tags, or node types)"
                    }
                },
                "required": ["query"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let query_lower = args.query.to_lowercase();

        // Filter matching templates, excluding current template being edited
        let mut matches: Vec<&TemplateInfo> = self
            .templates
            .iter()
            .filter(|t| {
                // Skip the current template being edited
                if let Some(ref current_id) = self.current_template_id {
                    if &t.id == current_id {
                        return false;
                    }
                }
                t.name.to_lowercase().contains(&query_lower)
                    || t.description.to_lowercase().contains(&query_lower)
                    || t.tags
                        .iter()
                        .any(|tag| tag.to_lowercase().contains(&query_lower))
                    || t.node_types
                        .iter()
                        .any(|nt| nt.to_lowercase().contains(&query_lower))
            })
            .take(5) // Limit results to reduce context
            .collect();

        // Sort by relevance: exact name match first, then description match
        matches.sort_by(|a, b| {
            let a_name_match = a.name.to_lowercase().contains(&query_lower);
            let b_name_match = b.name.to_lowercase().contains(&query_lower);
            b_name_match.cmp(&a_name_match)
        });

        Ok(serde_json::to_string(&matches).unwrap_or_default())
    }
}

struct FocusNodeTool;

impl Tool for FocusNodeTool {
    const NAME: &'static str = "focus_node";

    type Error = FocusNodeToolError;
    type Args = FocusNodeArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "focus_node".to_string(),
            description: "When explaining the board or referencing specific nodes, use this to highlight and focus on a particular node. The UI will automatically navigate to and highlight the node.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "node_id": {
                        "type": "string",
                        "description": "The ID of the node to focus on"
                    },
                    "description": {
                        "type": "string",
                        "description": "Brief description of why you're focusing on this node (e.g., 'This node processes the user input')"
                    }
                },
                "required": ["node_id", "description"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        Ok(format!("<focus_node>{}</focus_node>", args.node_id))
    }
}

struct EmitCommandsTool;

impl Tool for EmitCommandsTool {
    const NAME: &'static str = "emit_commands";

    type Error = EmitCommandsToolError;
    type Args = EmitCommandsArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "emit_commands".to_string(),
            description: "Emit a list of commands to modify the graph. Each command MUST include a 'summary' field with a brief human-readable description. Commands are executed by the frontend to maintain undo/redo history.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "commands": {
                        "type": "array",
                        "description": "List of commands to execute. Each command MUST have a 'summary' field.",
                        "items": {
                            "type": "object",
                            "oneOf": [
                                {
                                    "properties": {
                                        "command_type": { "const": "AddNode" },
                                        "node_type": { "type": "string", "description": "The node type from the catalog" },
                                        "ref_id": { "type": "string", "description": "Reference ID for this node (e.g., '$0', '$1') to use in subsequent commands" },
                                        "position": {
                                            "type": "object",
                                            "properties": { "x": { "type": "number" }, "y": { "type": "number" } }
                                        },
                                        "friendly_name": { "type": "string", "description": "Optional friendly name" },
                                        "summary": { "type": "string", "description": "Human-readable summary, e.g. 'Add a String Concat node'" }
                                    },
                                    "required": ["command_type", "node_type", "ref_id", "summary"]
                                },
                                {
                                    "properties": {
                                        "command_type": { "const": "RemoveNode" },
                                        "node_id": { "type": "string", "description": "The ID of the node to remove" },
                                        "summary": { "type": "string", "description": "Human-readable summary, e.g. 'Remove the unused filter node'" }
                                    },
                                    "required": ["command_type", "node_id", "summary"]
                                },
                                {
                                    "properties": {
                                        "command_type": { "const": "ConnectPins" },
                                        "from_node": { "type": "string", "description": "Source node ID or ref_id (e.g., '$0')" },
                                        "from_pin": { "type": "string", "description": "Output pin NAME (not ID)" },
                                        "to_node": { "type": "string", "description": "Target node ID or ref_id (e.g., '$1')" },
                                        "to_pin": { "type": "string", "description": "Input pin NAME (not ID)" },
                                        "summary": { "type": "string", "description": "Human-readable summary, e.g. 'Connect output to input'" }
                                    },
                                    "required": ["command_type", "from_node", "from_pin", "to_node", "to_pin", "summary"]
                                },
                                {
                                    "properties": {
                                        "command_type": { "const": "DisconnectPins" },
                                        "from_node": { "type": "string" },
                                        "from_pin": { "type": "string" },
                                        "to_node": { "type": "string" },
                                        "to_pin": { "type": "string" },
                                        "summary": { "type": "string", "description": "Human-readable summary" }
                                    },
                                    "required": ["command_type", "from_node", "from_pin", "to_node", "to_pin", "summary"]
                                },
                                {
                                    "properties": {
                                        "command_type": { "const": "UpdateNodePin" },
                                        "node_id": { "type": "string", "description": "Node ID or ref_id (e.g., '$0')" },
                                        "pin_id": { "type": "string", "description": "Pin NAME (use internal name from catalog, not friendly_name)" },
                                        "value": { "description": "The new value for the pin" },
                                        "summary": { "type": "string", "description": "Human-readable summary, e.g. 'Set threshold to 0.5'" }
                                    },
                                    "required": ["command_type", "node_id", "pin_id", "value", "summary"]
                                },
                                {
                                    "properties": {
                                        "command_type": { "const": "MoveNode" },
                                        "node_id": { "type": "string" },
                                        "position": {
                                            "type": "object",
                                            "properties": { "x": { "type": "number" }, "y": { "type": "number" } },
                                            "required": ["x", "y"]
                                        },
                                        "summary": { "type": "string", "description": "Human-readable summary" }
                                    },
                                    "required": ["command_type", "node_id", "position", "summary"]
                                },
                                {
                                    "properties": {
                                        "command_type": { "const": "CreateVariable" },
                                        "name": { "type": "string", "description": "Variable name" },
                                        "data_type": { "type": "string", "description": "Data type: String, Integer, Float, Boolean, Struct, etc." },
                                        "value_type": { "type": "string", "description": "Value type: Normal, Array, HashMap, HashSet" },
                                        "default_value": { "description": "Optional default value" },
                                        "description": { "type": "string", "description": "Optional description" },
                                        "summary": { "type": "string", "description": "Human-readable summary" }
                                    },
                                    "required": ["command_type", "name", "data_type", "summary"]
                                },
                                {
                                    "properties": {
                                        "command_type": { "const": "UpdateVariable" },
                                        "variable_id": { "type": "string", "description": "Variable ID from context" },
                                        "value": { "description": "New value for the variable" },
                                        "summary": { "type": "string", "description": "Human-readable summary" }
                                    },
                                    "required": ["command_type", "variable_id", "value", "summary"]
                                },
                                {
                                    "properties": {
                                        "command_type": { "const": "DeleteVariable" },
                                        "variable_id": { "type": "string", "description": "Variable ID from context" },
                                        "summary": { "type": "string", "description": "Human-readable summary" }
                                    },
                                    "required": ["command_type", "variable_id", "summary"]
                                },
                                {
                                    "properties": {
                                        "command_type": { "const": "CreateComment" },
                                        "content": { "type": "string", "description": "Comment text" },
                                        "position": {
                                            "type": "object",
                                            "properties": { "x": { "type": "number" }, "y": { "type": "number" } }
                                        },
                                        "color": { "type": "string", "description": "Optional color" },
                                        "summary": { "type": "string", "description": "Human-readable summary" }
                                    },
                                    "required": ["command_type", "content", "summary"]
                                },
                                {
                                    "properties": {
                                        "command_type": { "const": "UpdateComment" },
                                        "comment_id": { "type": "string", "description": "Comment ID from context" },
                                        "content": { "type": "string", "description": "New content" },
                                        "color": { "type": "string", "description": "New color" },
                                        "summary": { "type": "string", "description": "Human-readable summary" }
                                    },
                                    "required": ["command_type", "comment_id", "summary"]
                                },
                                {
                                    "properties": {
                                        "command_type": { "const": "DeleteComment" },
                                        "comment_id": { "type": "string", "description": "Comment ID from context" },
                                        "summary": { "type": "string", "description": "Human-readable summary" }
                                    },
                                    "required": ["command_type", "comment_id", "summary"]
                                },
                                {
                                    "properties": {
                                        "command_type": { "const": "CreateLayer" },
                                        "name": { "type": "string", "description": "Layer name" },
                                        "color": { "type": "string", "description": "Optional layer color" },
                                        "node_ids": { "type": "array", "items": { "type": "string" }, "description": "Node IDs to include" },
                                        "summary": { "type": "string", "description": "Human-readable summary" }
                                    },
                                    "required": ["command_type", "name", "summary"]
                                },
                                {
                                    "properties": {
                                        "command_type": { "const": "AddNodesToLayer" },
                                        "layer_id": { "type": "string", "description": "Layer ID from context" },
                                        "node_ids": { "type": "array", "items": { "type": "string" }, "description": "Node IDs to add" },
                                        "summary": { "type": "string", "description": "Human-readable summary" }
                                    },
                                    "required": ["command_type", "layer_id", "node_ids", "summary"]
                                },
                                {
                                    "properties": {
                                        "command_type": { "const": "RemoveNodesFromLayer" },
                                        "layer_id": { "type": "string", "description": "Layer ID from context" },
                                        "node_ids": { "type": "array", "items": { "type": "string" }, "description": "Node IDs to remove" },
                                        "summary": { "type": "string", "description": "Human-readable summary" }
                                    },
                                    "required": ["command_type", "layer_id", "node_ids", "summary"]
                                }
                            ]
                        }
                    },
                    "explanation": {
                        "type": "string",
                        "description": "Overall explanation of what these commands accomplish together"
                    }
                },
                "required": ["commands", "explanation"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Return the commands as JSON wrapped in a special tag for parsing
        let commands_json = serde_json::to_string(&args.commands).unwrap_or_default();
        Ok(format!(
            "<commands>{}</commands>\n\n{}",
            commands_json, args.explanation
        ))
    }
}

use futures::StreamExt;
use rig::{
    OneOrMany,
    completion::Completion,
    message::{
        AssistantContent, ToolCall as RigToolCall, ToolResult as RigToolResult, ToolResultContent,
        UserContent,
    },
    streaming::StreamedAssistantContent,
};

pub struct Copilot {
    state: FlowLikeState,
    catalog_provider: Arc<dyn CatalogProvider>,
    profile: Option<Arc<Profile>>,
    templates: Vec<TemplateInfo>,
    /// Current template ID if editing a template (prioritized in search)
    current_template_id: Option<String>,
}

impl Copilot {
    /// Create a new Copilot - always loads templates from profile
    pub async fn new(
        state: FlowLikeState,
        catalog_provider: Arc<dyn CatalogProvider>,
        profile: Option<Arc<Profile>>,
        current_template_id: Option<String>,
    ) -> Result<Self> {
        let templates = if let Some(ref profile) = profile {
            Self::load_templates_from_profile(&state, profile)
                .await
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        Ok(Self {
            state,
            catalog_provider,
            profile,
            templates,
            current_template_id,
        })
    }

    /// Load all templates from the user's profile apps
    async fn load_templates_from_profile(
        state: &FlowLikeState,
        profile: &Profile,
    ) -> Result<Vec<TemplateInfo>> {
        let mut templates = Vec::new();

        let app_ids: Vec<String> = profile
            .apps
            .as_ref()
            .map(|apps| apps.iter().map(|a| a.app_id.clone()).collect())
            .unwrap_or_default();

        let state_arc = Arc::new(Mutex::new(state.clone()));

        for app_id in app_ids {
            // Try to load the app
            let app = match App::load(app_id.clone(), state_arc.clone()).await {
                Ok(app) => app,
                Err(_) => continue,
            };

            // Load templates from this app
            for template_id in &app.templates {
                let template_info = match Self::load_template_info(&app, template_id).await {
                    Ok(info) => info,
                    Err(_) => continue,
                };
                templates.push(template_info);
            }
        }

        Ok(templates)
    }

    /// Load template info (metadata + structure analysis)
    async fn load_template_info(app: &App, template_id: &str) -> Result<TemplateInfo> {
        // Get template metadata
        let meta = app
            .get_template_meta(template_id, None)
            .await
            .unwrap_or_else(|_| Metadata::default());

        // Load the template board to analyze its structure
        let template_board = app.open_template(template_id.to_string(), None).await?;

        // Extract unique node types used in this template
        let node_types: Vec<String> = template_board
            .nodes
            .values()
            .map(|n| n.name.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .take(10) // Limit to avoid bloating context
            .collect();

        Ok(TemplateInfo {
            id: template_id.to_string(),
            app_id: app.id.clone(),
            name: meta.name,
            description: meta.description,
            tags: meta.tags,
            node_count: template_board.nodes.len(),
            node_types,
        })
    }

    /// Main entry point - unified agent that can both explain and modify
    pub async fn chat<F>(
        &self,
        board: &Board,
        selected_node_ids: &[String],
        user_prompt: String,
        history: Vec<ChatMessage>,
        model_id: Option<String>,
        token: Option<String>,
        on_token: Option<F>,
    ) -> Result<CopilotResponse>
    where
        F: Fn(String) + Send + Sync + 'static,
    {
        let context = self.prepare_context(board, selected_node_ids).await?;
        let context_json = flow_like_types::json::to_string_pretty(&context)?;

        // Only include node type names (not full paths) for context efficiency
        let available_nodes = self.catalog_provider.get_all_nodes().await;
        let node_count = available_nodes.len();

        let (model_name, completion_client) = self.get_model(model_id, token).await?;

        // Build a compact system prompt
        let system_prompt =
            Self::build_system_prompt(&context_json, node_count, !self.templates.is_empty());

        let mut agent_builder = completion_client
            .agent(&model_name)
            .preamble(&system_prompt)
            .tool(ThinkTool)
            .tool(FocusNodeTool)
            .tool(EmitCommandsTool)
            .tool(CatalogTool {
                provider: self.catalog_provider.clone(),
            })
            .tool(SearchByPinTool {
                provider: self.catalog_provider.clone(),
            })
            .tool(FilterCategoryTool {
                provider: self.catalog_provider.clone(),
            });

        // Only add templates tool if we have templates
        if !self.templates.is_empty() {
            agent_builder = agent_builder.tool(SearchTemplatesTool {
                templates: self.templates.clone(),
                current_template_id: self.current_template_id.clone(),
            });
        }

        let agent = agent_builder.build();

        let prompt = user_prompt.clone();

        // Convert chat history to rig message format
        let mut current_history: Vec<rig::message::Message> = history
            .iter()
            .map(|msg| match msg.role {
                ChatRole::User => rig::message::Message::User {
                    content: OneOrMany::one(UserContent::Text(rig::message::Text {
                        text: msg.content.clone(),
                    })),
                },
                ChatRole::Assistant => rig::message::Message::Assistant {
                    id: None,
                    content: OneOrMany::one(AssistantContent::Text(rig::message::Text {
                        text: msg.content.clone(),
                    })),
                },
            })
            .collect();

        let mut full_response = String::new();
        let mut all_commands: Vec<BoardCommand> = Vec::new();
        let max_iterations = 10u64;
        let mut plan_step_counter = 0u32;

        for iteration in 0..max_iterations {
            // Build completion request - tools are already attached via agent builder
            let request = agent
                .completion(prompt.clone(), current_history.clone())
                .await
                .map_err(|e| flow_like_types::anyhow!("Completion error: {}", e))?;

            // Stream the response
            let mut stream = request
                .stream()
                .await
                .map_err(|e| flow_like_types::anyhow!("Stream error: {}", e))?;

            let mut response_contents: Vec<AssistantContent> = Vec::new();
            let mut iteration_text = String::new();
            let mut current_reasoning = String::new();
            let mut reasoning_step_id: Option<String> = None;

            while let Some(item) = stream.next().await {
                let content =
                    item.map_err(|e| flow_like_types::anyhow!("Stream chunk error: {}", e))?;

                match content {
                    StreamedAssistantContent::Text(text) => {
                        iteration_text.push_str(&text.text);
                        if let Some(ref callback) = on_token {
                            callback(text.text.clone());
                        }
                        response_contents.push(AssistantContent::Text(text));
                    }
                    StreamedAssistantContent::ToolCall(tool_call) => {
                        response_contents.push(AssistantContent::ToolCall(tool_call));
                    }
                    StreamedAssistantContent::ToolCallDelta { .. } => {
                        // Deltas are accumulated into the final ToolCall
                    }
                    StreamedAssistantContent::Reasoning(reasoning) => {
                        let reasoning_text = reasoning.reasoning.join("\n");
                        current_reasoning.push_str(&reasoning_text);
                        current_reasoning.push('\n');

                        // Send reasoning as a plan step (streaming update)
                        if let Some(ref callback) = on_token {
                            // Create or update the reasoning step
                            if reasoning_step_id.is_none() {
                                plan_step_counter += 1;
                                reasoning_step_id =
                                    Some(format!("reasoning_{}", plan_step_counter));
                            }

                            let step_event = StreamEvent::PlanStep(PlanStep {
                                id: reasoning_step_id.clone().unwrap(),
                                description: current_reasoning.trim().to_string(),
                                status: PlanStepStatus::InProgress,
                                tool_name: Some("think".to_string()),
                            });
                            callback(format!(
                                "<plan_step>{}</plan_step>",
                                serde_json::to_string(&step_event).unwrap_or_default()
                            ));
                        }
                    }
                    StreamedAssistantContent::Final(_) => {
                        // Mark reasoning step as completed
                        if let (Some(callback), Some(step_id)) = (&on_token, &reasoning_step_id) {
                            let step_event = StreamEvent::PlanStep(PlanStep {
                                id: step_id.clone(),
                                description: current_reasoning.trim().to_string(),
                                status: PlanStepStatus::Completed,
                                tool_name: Some("think".to_string()),
                            });
                            callback(format!(
                                "<plan_step>{}</plan_step>",
                                serde_json::to_string(&step_event).unwrap_or_default()
                            ));
                        }
                        reasoning_step_id = None;
                        current_reasoning.clear();
                    }
                }
            }

            // Mark reasoning step as completed if stream ended while reasoning
            if let (Some(callback), Some(step_id)) = (&on_token, &reasoning_step_id) {
                let step_event = StreamEvent::PlanStep(PlanStep {
                    id: step_id.clone(),
                    description: current_reasoning.trim().to_string(),
                    status: PlanStepStatus::Completed,
                    tool_name: Some("think".to_string()),
                });
                callback(format!(
                    "<plan_step>{}</plan_step>",
                    serde_json::to_string(&step_event).unwrap_or_default()
                ));
            }

            full_response.push_str(&iteration_text);

            // Collect all tool calls first for parallel execution
            let tool_calls: Vec<_> = response_contents
                .iter()
                .filter_map(|content| {
                    if let AssistantContent::ToolCall(tool_call) = content {
                        Some(tool_call.clone())
                    } else {
                        None
                    }
                })
                .collect();

            let tool_calls_found = !tool_calls.is_empty();

            if tool_calls_found {
                // Emit plan steps for all tool calls starting
                let mut step_ids: Vec<(String, String, u32)> = Vec::new();
                for tool_call in &tool_calls {
                    plan_step_counter += 1;
                    let step_id = format!("step_{}", plan_step_counter);
                    let step_description = Self::get_tool_description(
                        &tool_call.function.name,
                        &tool_call.function.arguments,
                    );

                    if let Some(ref callback) = on_token {
                        callback(format!("tool_call:{}", tool_call.function.name));
                        let step_event = StreamEvent::PlanStep(PlanStep {
                            id: step_id.clone(),
                            description: step_description.clone(),
                            status: PlanStepStatus::InProgress,
                            tool_name: Some(tool_call.function.name.clone()),
                        });
                        callback(format!(
                            "<plan_step>{}</plan_step>",
                            serde_json::to_string(&step_event).unwrap_or_default()
                        ));
                    }

                    step_ids.push((step_id, step_description, plan_step_counter));
                }

                // Execute all tools in parallel
                let tool_futures: Vec<_> = tool_calls
                    .iter()
                    .map(|tool_call| {
                        let name = tool_call.function.name.clone();
                        let arguments = tool_call.function.arguments.clone();
                        let id = tool_call.id.clone();
                        async move {
                            let output = self.execute_tool(&name, &arguments).await;
                            (id, name, output)
                        }
                    })
                    .collect();

                let tool_results: Vec<(String, String, String)> =
                    futures::future::join_all(tool_futures).await;

                // Process results and emit completion events
                for (i, (id, name, tool_output)) in tool_results.iter().enumerate() {
                    println!(
                        "[Copilot] Tool '{}' (id={}) output length: {} chars",
                        name,
                        id,
                        tool_output.len()
                    );

                    // Parse commands from emit_commands tool output
                    if name == "emit_commands" {
                        let parsed = Self::parse_commands(tool_output);
                        println!("[Copilot] emit_commands parsed {} commands:", parsed.len());
                        for (idx, cmd) in parsed.iter().enumerate() {
                            println!("[Copilot]   [{}] {:?}", idx, cmd);
                        }
                        all_commands.extend(parsed);
                        println!(
                            "[Copilot] all_commands now has {} total commands",
                            all_commands.len()
                        );
                    }

                    // Emit plan step completion
                    if let Some(ref callback) = on_token {
                        if let Some((step_id, step_description, _)) = step_ids.get(i) {
                            let step_event = StreamEvent::PlanStep(PlanStep {
                                id: step_id.clone(),
                                description: step_description.clone(),
                                status: PlanStepStatus::Completed,
                                tool_name: Some(name.clone()),
                            });
                            callback(format!(
                                "<plan_step>{}</plan_step>",
                                serde_json::to_string(&step_event).unwrap_or_default()
                            ));
                        }
                        callback("tool_result:done".to_string());
                    }

                    // Only stream focus_node results to the user
                    if let Some(ref callback) = on_token {
                        if name == "focus_node" {
                            callback(tool_output.clone());
                        }
                    }
                }

                // Add assistant message with tool calls to history
                let assistant_msg = rig::message::Message::Assistant {
                    id: None,
                    content: OneOrMany::many(response_contents.clone()).unwrap_or_else(|_| {
                        OneOrMany::one(AssistantContent::Text(rig::message::Text {
                            text: String::new(),
                        }))
                    }),
                };
                current_history.push(assistant_msg);

                // Add tool results to history
                for (tool_id, _tool_name, tool_output) in &tool_results {
                    let tool_result_msg = rig::message::Message::User {
                        content: OneOrMany::one(UserContent::ToolResult(RigToolResult {
                            id: tool_id.clone(),
                            call_id: None,
                            content: OneOrMany::one(ToolResultContent::text(tool_output.clone())),
                        })),
                    };
                    current_history.push(tool_result_msg);
                }
            } else {
                // No tool calls, we're done
                break;
            }

            // Continue to next iteration (agent will see tool results and continue)
            if iteration == max_iterations - 1 {
                break;
            }
        }

        let has_commands = !all_commands.is_empty();
        println!(
            "[Copilot] Final response: {} total commands, agent_type={:?}",
            all_commands.len(),
            if has_commands {
                AgentType::Edit
            } else {
                AgentType::Explain
            }
        );

        // Log the serialized response for debugging
        let response = CopilotResponse {
            agent_type: if has_commands {
                AgentType::Edit
            } else {
                AgentType::Explain
            },
            message: Self::clean_message(&full_response),
            commands: all_commands,
            suggestions: vec![],
        };

        if let Ok(json) = serde_json::to_string(&response) {
            println!("[Copilot] Response JSON length: {} chars", json.len());
            if response.commands.len() > 0 {
                println!(
                    "[Copilot] First command serialized: {:?}",
                    serde_json::to_string(&response.commands[0])
                );
            }
        }

        Ok(response)
    }

    /// Build a compact system prompt to reduce context size
    fn build_system_prompt(context_json: &str, node_count: usize, has_templates: bool) -> String {
        let templates_tool = if has_templates {
            "\n- **search_templates**: Search workflow templates for implementation examples"
        } else {
            ""
        };

        format!(
            r#"You are an expert graph editor assistant. You help users understand and modify visual workflows.

## Graph Context (abbreviated keys: t=type, n=name, i=inputs, o=outputs, p=position, s=size, f=from, fp=from_pin, tp=to_pin, v=value)
{}

## Tools
**Understanding**: think (reason step-by-step), focus_node (highlight node with <focus_node>id</focus_node>)
**Catalog** ({} nodes): catalog_search (by name/description), search_by_pin (by pin type), filter_category (by category){}
**Modify**: emit_commands (execute graph changes)

## Key Rules
1. Reference nodes with <focus_node>node_id</focus_node> in responses
2. Use pin `n` (name) in commands for pin connections
3. Connect compatible types only (check t=type from catalog)
4. New nodes need ref_id ("$0", "$1"...) for subsequent connections
5. Connect execution flow: exec_out → exec_in
6. Position nodes left-to-right, 250px horizontal spacing
7. Each command needs a `summary` field
8. Limit output to 20 commands per turn

## Commands
AddNode(node_type, ref_id, position, summary) | RemoveNode(node_id, summary)
ConnectPins(from_node, from_pin, to_node, to_pin, summary) | DisconnectPins(same)
UpdateNodePin(node_id, pin_id, value, summary) | MoveNode(node_id, position, summary)
CreateVariable(name, data_type, value_type, summary) | CreateComment(content, position, summary)

## Command Order
ALWAYS emit commands in this order:
1. AddNode commands first (create nodes)
2. ConnectPins commands (wire nodes together)
3. UpdateNodePin commands LAST (set default values)

## Workflow: Start from TARGET, work backwards. Search catalog first. Connect exec pins."#,
            context_json, node_count, templates_tool
        )
    }

    /// Get a human-readable description for a tool call
    fn get_tool_description(name: &str, arguments: &serde_json::Value) -> String {
        match name {
            "think" => {
                if let Some(thought) = arguments.get("thought").and_then(|v| v.as_str()) {
                    thought.to_string()
                } else {
                    "Reasoning through the problem...".to_string()
                }
            }
            "focus_node" => {
                if let Some(node_id) = arguments.get("node_id").and_then(|v| v.as_str()) {
                    format!("Examining node {}", node_id)
                } else {
                    "Examining a node...".to_string()
                }
            }
            "emit_commands" => {
                if let Some(commands) = arguments.get("commands").and_then(|v| v.as_array()) {
                    format!("Preparing {} change(s)...", commands.len())
                } else {
                    "Preparing changes...".to_string()
                }
            }
            "catalog_search" => {
                if let Some(query) = arguments.get("query").and_then(|v| v.as_str()) {
                    format!("Searching catalog for \"{}\"", query)
                } else {
                    "Searching the catalog...".to_string()
                }
            }
            "search_by_pin" => {
                if let Some(pin_type) = arguments.get("pin_type").and_then(|v| v.as_str()) {
                    format!("Finding nodes with {} pins", pin_type)
                } else {
                    "Finding compatible nodes...".to_string()
                }
            }
            "filter_category" => {
                if let Some(category) = arguments.get("category_prefix").and_then(|v| v.as_str()) {
                    format!("Browsing {} category", category)
                } else {
                    "Browsing categories...".to_string()
                }
            }
            "search_templates" => {
                if let Some(query) = arguments.get("query").and_then(|v| v.as_str()) {
                    format!("Searching templates for \"{}\"", query)
                } else {
                    "Searching templates...".to_string()
                }
            }
            _ => format!("Running {}...", name),
        }
    }

    /// Execute a tool by name and return the result
    async fn execute_tool(&self, name: &str, arguments: &serde_json::Value) -> String {
        match name {
            "think" => {
                if let Ok(args) = serde_json::from_value::<ThinkingArgs>(arguments.clone()) {
                    format!("Thinking: {}", args.thought)
                } else {
                    "Thinking...".to_string()
                }
            }
            "focus_node" => {
                if let Ok(args) = serde_json::from_value::<FocusNodeArgs>(arguments.clone()) {
                    format!("<focus_node>{}</focus_node>", args.node_id)
                } else {
                    "".to_string()
                }
            }
            "emit_commands" => {
                match serde_json::from_value::<EmitCommandsArgs>(arguments.clone()) {
                    Ok(args) => {
                        let commands_json =
                            serde_json::to_string(&args.commands).unwrap_or_default();
                        println!(
                            "[Copilot] emit_commands: {} commands, json length: {} chars",
                            args.commands.len(),
                            commands_json.len()
                        );
                        format!(
                            "<commands>{}</commands>\n\n{}",
                            commands_json, args.explanation
                        )
                    }
                    Err(e) => {
                        println!("[Copilot] emit_commands: Failed to parse args: {:?}", e);
                        println!("[Copilot] emit_commands: Raw arguments: {:?}", arguments);
                        format!("Failed to parse commands: {}", e)
                    }
                }
            }
            "catalog_search" => {
                if let Ok(args) = serde_json::from_value::<SearchArgs>(arguments.clone()) {
                    let matches = self.catalog_provider.search(&args.query).await;
                    serde_json::to_string(&matches).unwrap_or_default()
                } else {
                    "[]".to_string()
                }
            }
            "search_by_pin" => {
                if let Ok(args) = serde_json::from_value::<SearchByPinArgs>(arguments.clone()) {
                    let matches = self
                        .catalog_provider
                        .search_by_pin_type(&args.pin_type, args.is_input)
                        .await;
                    serde_json::to_string(&matches).unwrap_or_default()
                } else {
                    "[]".to_string()
                }
            }
            "filter_category" => {
                if let Ok(args) = serde_json::from_value::<FilterCategoryArgs>(arguments.clone()) {
                    let matches = self
                        .catalog_provider
                        .filter_by_category(&args.category_prefix)
                        .await;
                    serde_json::to_string(&matches).unwrap_or_default()
                } else {
                    "[]".to_string()
                }
            }
            "search_templates" => {
                if let Ok(args) = serde_json::from_value::<SearchTemplatesArgs>(arguments.clone()) {
                    let query_lower = args.query.to_lowercase();
                    let mut matches: Vec<&TemplateInfo> = self
                        .templates
                        .iter()
                        .filter(|t| {
                            // Skip current template being edited
                            if let Some(ref current_id) = self.current_template_id {
                                if &t.id == current_id {
                                    return false;
                                }
                            }
                            t.name.to_lowercase().contains(&query_lower)
                                || t.description.to_lowercase().contains(&query_lower)
                                || t.tags
                                    .iter()
                                    .any(|tag| tag.to_lowercase().contains(&query_lower))
                                || t.node_types
                                    .iter()
                                    .any(|nt| nt.to_lowercase().contains(&query_lower))
                        })
                        .take(5)
                        .collect();
                    // Sort by relevance
                    matches.sort_by(|a, b| {
                        let a_name_match = a.name.to_lowercase().contains(&query_lower);
                        let b_name_match = b.name.to_lowercase().contains(&query_lower);
                        b_name_match.cmp(&a_name_match)
                    });
                    serde_json::to_string(&matches).unwrap_or_default()
                } else {
                    "[]".to_string()
                }
            }
            _ => format!("Unknown tool: {}", name),
        }
    }

    /// Parse commands from the agent's response
    fn parse_commands(response: &str) -> Vec<BoardCommand> {
        // Look for <commands>...</commands> tags
        if let Some(start) = response.find("<commands>") {
            if let Some(end) = response.find("</commands>") {
                let json_str = &response[start + 10..end];
                if let Ok(commands) = serde_json::from_str::<Vec<BoardCommand>>(json_str) {
                    return commands;
                }
            }
        }
        vec![]
    }

    /// Clean the message by removing command tags
    fn clean_message(response: &str) -> String {
        // Remove <commands>...</commands> block
        let mut result = response.to_string();
        if let Some(start) = result.find("<commands>") {
            if let Some(end) = result.find("</commands>") {
                result = format!("{}{}", &result[..start], &result[end + 11..]);
            }
        }
        result.trim().to_string()
    }

    /// Get the model for the agent
    async fn get_model<'a>(
        &self,
        model_id: Option<String>,
        token: Option<String>,
    ) -> Result<(String, Box<dyn CompletionClientDyn + 'a>)> {
        let bit = if let Some(profile) = &self.profile {
            if let Some(id) = model_id {
                profile
                    .find_bit(&id, self.state.http_client.clone())
                    .await?
            } else {
                let preference = BitModelPreference {
                    reasoning_weight: Some(1.0),
                    ..Default::default()
                };
                profile
                    .get_best_model(&preference, false, true, self.state.http_client.clone())
                    .await?
            }
        } else {
            Bit {
                id: "gpt-4o".to_string(),
                bit_type: BitTypes::Llm,
                parameters: serde_json::to_value(LLMParameters {
                    context_length: 128000,
                    provider: ModelProvider {
                        provider_name: "openai".to_string(),
                        model_id: None,
                        version: None,
                        params: None,
                    },
                    model_classification: Default::default(),
                })
                .unwrap(),
                ..Default::default()
            }
        };

        let model_factory = self.state.model_factory.clone();
        let model = model_factory
            .lock()
            .await
            .build(&bit, Arc::new(Mutex::new(self.state.clone())), token)
            .await?;
        let default_model = model.default_model().await.unwrap_or("gpt-4o".to_string());
        let provider = model.provider().await?;
        let client = provider.client();
        let completion = client
            .as_completion()
            .ok_or_else(|| flow_like_types::anyhow!("Model does not support completion"))?;

        Ok((default_model, completion))
    }

    async fn prepare_context(
        &self,
        board: &Board,
        selected_node_ids: &[String],
    ) -> Result<GraphContext> {
        let mut node_contexts = Vec::new();
        let mut pin_to_node_map = std::collections::HashMap::new();

        // Helper to process nodes
        let mut process_nodes = |nodes: &std::collections::HashMap<String, Node>| {
            for node in nodes.values() {
                for pin_id in node.pins.keys() {
                    pin_to_node_map.insert(pin_id.clone(), node.id.clone());
                }
            }
        };

        // Build pin to node map for root nodes
        process_nodes(&board.nodes);
        // Build pin to node map for layer nodes
        for layer in board.layers.values() {
            process_nodes(&layer.nodes);
        }

        // Helper to create context
        let mut create_node_contexts = |nodes: &std::collections::HashMap<String, Node>| {
            for node in nodes.values() {
                // Only include non-execution pins with meaningful info
                let inputs: Vec<PinContext> = node
                    .pins
                    .iter()
                    .filter(|(_, p)| p.pin_type == PinType::Input)
                    .map(|(_, p)| {
                        let default_val = p
                            .default_value
                            .as_ref()
                            .map(|v| String::from_utf8_lossy(v).to_string())
                            .filter(|s| !s.is_empty() && s != "null");
                        PinContext {
                            name: p.name.clone(),
                            type_name: format!("{:?}", p.data_type),
                            default_value: default_val,
                        }
                    })
                    .collect();

                let outputs: Vec<PinContext> = node
                    .pins
                    .iter()
                    .filter(|(_, p)| p.pin_type == PinType::Output)
                    .map(|(_, p)| PinContext {
                        name: p.name.clone(),
                        type_name: format!("{:?}", p.data_type),
                        default_value: None, // Outputs don't have default values
                    })
                    .collect();

                // Estimate node size based on pin count
                let input_count = inputs.len();
                let output_count = outputs.len();
                let max_pins = input_count.max(output_count);
                let estimated_width = 200u16;
                let estimated_height = 32u16 + (max_pins as u16 * 20);

                let (x, y) = node
                    .coordinates
                    .map(|(x, y, _)| (x as i32, y as i32))
                    .unwrap_or((0, 0));

                node_contexts.push(NodeContext {
                    id: node.id.clone(),
                    node_type: node.name.clone(),
                    friendly_name: node.friendly_name.clone(),
                    inputs,
                    outputs,
                    position: (x, y),
                    estimated_size: (estimated_width, estimated_height),
                });
            }
        };

        create_node_contexts(&board.nodes);
        for layer in board.layers.values() {
            create_node_contexts(&layer.nodes);
        }

        let mut edge_contexts = Vec::new();

        let mut process_edges = |nodes: &std::collections::HashMap<String, Node>| {
            for node in nodes.values() {
                for (_, pin) in &node.pins {
                    // We only care about outgoing connections to avoid duplicates
                    if pin.pin_type == PinType::Output {
                        for connected_pin_id in &pin.connected_to {
                            if let Some(target_node_id) = pin_to_node_map.get(connected_pin_id) {
                                let target_pin = board.get_pin_by_id(connected_pin_id);
                                edge_contexts.push(EdgeContext {
                                    from_node_id: node.id.clone(),
                                    from_pin_name: pin.name.clone(),
                                    to_node_id: target_node_id.clone(),
                                    to_pin_name: target_pin
                                        .map(|p| p.name.clone())
                                        .unwrap_or_default(),
                                });
                            }
                        }
                    }
                }
            }
        };

        process_edges(&board.nodes);
        for layer in board.layers.values() {
            process_edges(&layer.nodes);
        }

        Ok(GraphContext {
            nodes: node_contexts,
            edges: edge_contexts,
            selected_nodes: selected_node_ids.to_vec(),
        })
    }
}
