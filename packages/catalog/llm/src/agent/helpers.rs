use crate::generative::agent::{Agent, ContextManagementMode};
/// # Agent Execution Helpers
/// This module contains reusable logic for executing agents with tools and streaming.
/// Extracted from simple.rs to be shared across multiple agent nodes.
#[cfg(feature = "execute")]
use ahash::AHashSet;
#[cfg(feature = "execute")]
use flow_like::flow::execution::LogLevel;
use flow_like::flow::{
    execution::{context::ExecutionContext, internal_node::InternalNode},
    pin::PinType,
    variable::VariableType,
};
use flow_like_model_provider::{
    history::{Content, ContentType, History, HistoryMessage, MessageContent, Role, Tool},
    response::{Response, Usage as ResponseUsage},
    response_chunk::ResponseChunk,
};
use flow_like_types::{
    Value, anyhow, async_trait, json,
    sync::{DashMap, Mutex},
};
#[cfg(feature = "execute")]
use futures::StreamExt;
#[cfg(feature = "execute")]
use rig::OneOrMany;
#[cfg(feature = "execute")]
use rig::completion::{Completion, ToolDefinition, Usage as RigUsage};
#[cfg(feature = "execute")]
use rig::message::{AssistantContent, ToolCall as RigToolCall};
#[cfg(feature = "execute")]
use rig::streaming::StreamedAssistantContent;
#[cfg(feature = "execute")]
use rig::tools::ThinkTool;
#[cfg(feature = "execute")]
use rmcp::{
    ServiceExt,
    model::{
        CallToolRequestParam, ClientCapabilities, ClientInfo, Implementation, PaginatedRequestParam,
    },
};
use std::{collections::HashMap, sync::Arc};

const DEFAULT_MAX_CONTEXT_TOKENS: u32 = 32000;
const CHARS_PER_TOKEN_ESTIMATE: usize = 4;

/// Estimate token count for a message using character-based heuristic.
/// Most LLMs average ~4 characters per token for English text.
#[cfg(feature = "execute")]
fn estimate_message_tokens(msg: &rig::message::Message) -> usize {
    let char_count: usize = match msg {
        rig::message::Message::User { content } => content
            .iter()
            .map(|c| match c {
                rig::message::UserContent::Text(t) => t.text.len(),
                rig::message::UserContent::ToolResult(tr) => tr
                    .content
                    .iter()
                    .map(|trc| match trc {
                        rig::message::ToolResultContent::Text(t) => t.text.len(),
                        _ => 50,
                    })
                    .sum(),
                _ => 100,
            })
            .sum(),
        rig::message::Message::Assistant { content, .. } => content
            .iter()
            .map(|c| match c {
                AssistantContent::Text(t) => t.text.len(),
                AssistantContent::ToolCall(tc) => {
                    tc.function.name.len() + tc.function.arguments.to_string().len()
                }
                _ => 50,
            })
            .sum(),
    };
    (char_count / CHARS_PER_TOKEN_ESTIMATE).max(1) + 4 // +4 for message overhead
}

/// Truncate message history using sliding window to fit within token budget.
/// Preserves most recent messages while keeping tool call/result pairs intact.
/// Returns (truncated_history, truncated_count) where truncated_count is the number of removed messages.
#[cfg(feature = "execute")]
fn truncate_history_to_budget(
    history: Vec<rig::message::Message>,
    max_tokens: u32,
) -> (Vec<rig::message::Message>, usize) {
    if history.is_empty() {
        return (history, 0);
    }

    let total_tokens: usize = history.iter().map(estimate_message_tokens).sum();

    if total_tokens <= max_tokens as usize {
        return (history, 0);
    }

    let mut result = Vec::new();
    let mut current_tokens: usize = 0;
    let target_tokens = max_tokens as usize;

    // Track tool call IDs to keep pairs together
    let mut required_tool_ids: std::collections::HashSet<String> = std::collections::HashSet::new();

    // First pass: from end, collect messages until budget
    for msg in history.iter().rev() {
        let msg_tokens = estimate_message_tokens(msg);

        // Check for tool results - we need the corresponding tool call
        if let rig::message::Message::User { content } = msg {
            for c in content.iter() {
                if let rig::message::UserContent::ToolResult(tr) = c {
                    required_tool_ids.insert(tr.id.clone());
                }
            }
        }

        if current_tokens + msg_tokens <= target_tokens {
            result.push(msg.clone());
            current_tokens += msg_tokens;

            // Track tool calls so we don't orphan results
            if let rig::message::Message::Assistant { content, .. } = msg {
                for c in content.iter() {
                    if let AssistantContent::ToolCall(tc) = c {
                        required_tool_ids.remove(&tc.id);
                    }
                }
            }
        } else if !required_tool_ids.is_empty() {
            // Include anyway if we have orphaned tool results
            if let rig::message::Message::Assistant { content, .. } = msg {
                let has_required = content.iter().any(|c| {
                    if let AssistantContent::ToolCall(tc) = c {
                        required_tool_ids.contains(&tc.id)
                    } else {
                        false
                    }
                });
                if has_required {
                    result.push(msg.clone());
                    current_tokens += msg_tokens;
                    for c in content.iter() {
                        if let AssistantContent::ToolCall(tc) = c {
                            required_tool_ids.remove(&tc.id);
                        }
                    }
                    continue;
                }
            }
            break;
        } else {
            break;
        }
    }

    result.reverse();

    let truncated_count = history.len() - result.len();
    (result, truncated_count)
}

/// Summarize old messages using LLM to compress context while preserving key information.
/// Returns (updated_history, summarized_count) where summarized_count is messages compressed.
#[cfg(feature = "execute")]
async fn summarize_history_to_budget(
    context: &mut ExecutionContext,
    agent: &Agent,
    history: Vec<rig::message::Message>,
    max_tokens: u32,
) -> flow_like_types::Result<(Vec<rig::message::Message>, usize)> {
    if history.is_empty() {
        return Ok((history, 0));
    }

    let total_tokens: usize = history.iter().map(estimate_message_tokens).sum();

    if total_tokens <= max_tokens as usize {
        return Ok((history, 0));
    }

    // Find the split point: keep recent messages, summarize older ones
    // We want to keep ~60% budget for recent, ~40% for summary
    let recent_budget = (max_tokens as usize * 60) / 100;
    let mut recent_tokens: usize = 0;
    let mut split_idx = history.len();

    for (idx, msg) in history.iter().enumerate().rev() {
        let msg_tokens = estimate_message_tokens(msg);
        if recent_tokens + msg_tokens > recent_budget {
            split_idx = idx + 1;
            break;
        }
        recent_tokens += msg_tokens;
    }

    // If split would leave nothing to summarize, fall back to truncation
    if split_idx <= 1 {
        return Ok(truncate_history_to_budget(history, max_tokens));
    }

    let (old_messages, recent_messages) = history.split_at(split_idx);

    if old_messages.is_empty() {
        return Ok((recent_messages.to_vec(), 0));
    }

    // Convert old messages to text for summarization
    let mut conversation_text = String::new();
    for msg in old_messages {
        match msg {
            rig::message::Message::User { content } => {
                conversation_text.push_str("User: ");
                for c in content.iter() {
                    match c {
                        rig::message::UserContent::Text(t) => {
                            conversation_text.push_str(&t.text);
                        }
                        rig::message::UserContent::ToolResult(tr) => {
                            conversation_text.push_str(&format!("[Tool Result {}]", tr.id));
                        }
                        _ => {}
                    }
                }
                conversation_text.push('\n');
            }
            rig::message::Message::Assistant { content, .. } => {
                conversation_text.push_str("Assistant: ");
                for c in content.iter() {
                    match c {
                        AssistantContent::Text(t) => {
                            conversation_text.push_str(&t.text);
                        }
                        AssistantContent::ToolCall(tc) => {
                            conversation_text
                                .push_str(&format!("[Called tool: {}]", tc.function.name));
                        }
                        _ => {}
                    }
                }
                conversation_text.push('\n');
            }
        }
    }

    // Use the agent's model to generate a summary
    let summary_prompt = format!(
        "Summarize the following conversation history concisely, preserving key facts, decisions, and context that would be important for continuing the conversation. Focus on: user goals, important information shared, actions taken, and outcomes.\n\n---\n{}\n---\n\nProvide a concise summary:",
        conversation_text
    );

    let summary = match agent.model.agent(context, &None).await {
        Ok(agent_builder) => {
            let summary_agent = agent_builder
                .preamble(
                    "You are a conversation summarizer. Be concise but preserve key information.",
                )
                .build();

            match summary_agent.completion(summary_prompt, vec![]).await {
                Ok(request) => match request.send().await {
                    Ok(response) => {
                        // Extract text from response.choice
                        let mut text = String::new();
                        for content in response.choice {
                            if let AssistantContent::Text(t) = content {
                                text.push_str(&t.text);
                            }
                        }
                        if text.is_empty() {
                            context.log_message(
                                "Summary response was empty, falling back to truncation",
                                LogLevel::Warn,
                            );
                            return Ok(truncate_history_to_budget(history, max_tokens));
                        }
                        text
                    }
                    Err(e) => {
                        context.log_message(
                            &format!("Failed to get summary response: {}", e),
                            LogLevel::Warn,
                        );
                        // Fall back to truncation
                        return Ok(truncate_history_to_budget(history, max_tokens));
                    }
                },
                Err(e) => {
                    context.log_message(
                        &format!("Failed to create summary completion: {}", e),
                        LogLevel::Warn,
                    );
                    return Ok(truncate_history_to_budget(history, max_tokens));
                }
            }
        }
        Err(e) => {
            context.log_message(
                &format!("Failed to create summary agent: {}", e),
                LogLevel::Warn,
            );
            return Ok(truncate_history_to_budget(history, max_tokens));
        }
    };

    // Create a summary message to prepend
    let summary_msg = rig::message::Message::User {
        content: OneOrMany::one(rig::message::UserContent::Text(rig::message::Text {
            text: format!("[Previous conversation summary: {}]", summary),
        })),
    };

    // Combine: summary + recent messages
    let mut result = vec![summary_msg];
    result.extend(recent_messages.iter().cloned());

    let summarized_count = old_messages.len();
    Ok((result, summarized_count))
}

/// Manage context budget using the appropriate strategy (truncate or summarize).
/// Returns (managed_history, affected_count).
#[cfg(feature = "execute")]
async fn manage_context_budget(
    context: &mut ExecutionContext,
    agent: &Agent,
    history: Vec<rig::message::Message>,
    max_tokens: u32,
) -> flow_like_types::Result<(Vec<rig::message::Message>, usize)> {
    match agent.context_management_mode {
        ContextManagementMode::Summarize => {
            summarize_history_to_budget(context, agent, history, max_tokens).await
        }
        ContextManagementMode::Truncate => Ok(truncate_history_to_budget(history, max_tokens)),
    }
}

/// Generate OpenAI function call schema from a referenced function node.
/// Returns a Tool definition with function name, description, and parameter schema.
#[cfg(feature = "execute")]
pub async fn generate_tool_from_function(
    referenced_node: &Arc<InternalNode>,
) -> flow_like_types::Result<Tool> {
    use flow_like_model_provider::history::{
        HistoryFunction, HistoryFunctionParameters, HistoryJSONSchemaDefine, HistoryJSONSchemaType,
        ToolType,
    };
    use std::collections::HashMap;

    let node = referenced_node.node.lock().await;
    // Use friendly_name (user-customizable) and convert to snake_case for LLM
    let function_name = node.friendly_name.to_lowercase().replace([' ', '-'], "_");
    let description = node.description.clone();

    // Collect all non-execution output pins to build parameter schema
    let mut properties: HashMap<String, Box<HistoryJSONSchemaDefine>> = HashMap::new();
    let mut has_data_pins = false;
    let mut payload_pin = None;

    for (_pin_id, pin) in node.pins.iter() {
        // Skip execution pins and input pins
        if pin.data_type == VariableType::Execution || pin.pin_type != PinType::Output {
            continue;
        }

        // Track the payload pin separately
        if pin.name == "payload" {
            payload_pin = Some(pin);
            continue;
        }

        has_data_pins = true;

        // Map VariableType to JSONSchemaType
        let schema_type = match pin.data_type {
            VariableType::String => HistoryJSONSchemaType::String,
            VariableType::Integer => HistoryJSONSchemaType::Number,
            VariableType::Float => HistoryJSONSchemaType::Number,
            VariableType::Boolean => HistoryJSONSchemaType::Boolean,
            VariableType::Struct | VariableType::Generic => HistoryJSONSchemaType::Object,
            VariableType::Date | VariableType::PathBuf | VariableType::Byte => {
                HistoryJSONSchemaType::String
            }
            VariableType::Execution => continue, // Already filtered above
        };

        let property_def = HistoryJSONSchemaDefine {
            schema_type: Some(schema_type),
            description: if pin.description.is_empty() {
                None
            } else {
                Some(pin.description.clone())
            },
            enum_values: None,
            properties: None,
            required: None,
            items: None,
        };

        properties.insert(pin.name.clone(), Box::new(property_def));
    }

    // If no data pins exist AND the event has a payload pin defined, add it to the schema
    if !has_data_pins && let Some(payload) = payload_pin {
        let schema_type = match payload.data_type {
            VariableType::String => HistoryJSONSchemaType::String,
            VariableType::Integer => HistoryJSONSchemaType::Number,
            VariableType::Float => HistoryJSONSchemaType::Number,
            VariableType::Boolean => HistoryJSONSchemaType::Boolean,
            VariableType::Struct | VariableType::Generic => HistoryJSONSchemaType::Object,
            VariableType::Date | VariableType::PathBuf | VariableType::Byte => {
                HistoryJSONSchemaType::String
            }
            VariableType::Execution => HistoryJSONSchemaType::Object, // Fallback
        };

        let payload_def = HistoryJSONSchemaDefine {
            schema_type: Some(schema_type),
            description: if payload.description.is_empty() {
                None
            } else {
                Some(payload.description.clone())
            },
            enum_values: None,
            properties: None,
            required: None,
            items: None,
        };
        properties.insert("payload".to_string(), Box::new(payload_def));
    }

    let parameters = HistoryFunctionParameters {
        schema_type: HistoryJSONSchemaType::Object,
        properties: if properties.is_empty() {
            None
        } else {
            Some(properties)
        },
        required: None,
    };

    let function = HistoryFunction {
        name: function_name,
        description: if description.is_empty() {
            None
        } else {
            Some(description)
        },
        parameters,
    };

    Ok(Tool {
        tool_type: ToolType::Function,
        function,
    })
}

/// Execute a tool call by invoking the referenced function node with the provided arguments.
/// Returns the result as a JSON Value.
#[cfg(feature = "execute")]
pub async fn execute_tool_call(
    context: &mut ExecutionContext,
    referenced_node: &Arc<InternalNode>,
    tool_name: &str,
    arguments: &Value,
) -> flow_like_types::Result<Value> {
    context.log_message(
        &format!("Executing referenced function for tool {}", tool_name),
        LogLevel::Debug,
    );

    // Set the arguments as pin values on the referenced node
    let args_obj = arguments
        .as_object()
        .ok_or_else(|| anyhow!("Tool call arguments for '{}' are not an object", tool_name))?;

    // Set values on the referenced function's OUTPUT pins (matching call_ref.rs logic)
    for (_id, pin) in referenced_node.pins.iter() {
        // Skip input pins and execution pins
        if pin.pin_type == PinType::Input || pin.data_type == VariableType::Execution {
            continue;
        }

        // Set value if we have an argument for this pin
        if let Some(value) = args_obj.get(&pin.name) {
            pin.set_value(value.clone()).await;
        }
    }

    // Create a sub-context with the referenced node
    let mut sub_context = context.create_sub_context(referenced_node).await;
    sub_context.delegated = true;

    let run = InternalNode::trigger(&mut sub_context, &mut None, true).await;

    // CRITICAL: Capture result BEFORE end_trace and push_sub_context
    let captured_result = sub_context.result.clone();

    sub_context.end_trace();
    context.push_sub_context(&mut sub_context);

    match run {
        Ok(_) => {
            if let Some(ref result) = captured_result {
                Ok(result.clone())
            } else {
                Ok(json::json!("Tool executed successfully"))
            }
        }
        Err(error) => {
            context.log_message(
                &format!("Tool {} execution FAILED: {:?}", tool_name, error),
                LogLevel::Error,
            );
            Err(anyhow!("Tool execution failed: {:?}", error))
        }
    }
}

/// Agent execution result containing the final response and updated history
#[cfg(feature = "execute")]
pub struct AgentExecutionResult {
    pub response: Response,
    pub history: History,
}

/// Trait for handling stream emissions during agent execution
#[cfg(feature = "execute")]
#[async_trait]
pub trait StreamHandler: Send + Sync {
    async fn emit_chunk(
        &self,
        context: &mut ExecutionContext,
        chunk: &ResponseChunk,
    ) -> flow_like_types::Result<()>;

    async fn finalize(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()>;
}

/// Execute an agent with the given history and tool name mappings.
/// This is a non-streaming wrapper around execute_agent_streaming.
#[cfg(feature = "execute")]
pub async fn execute_agent(
    context: &mut ExecutionContext,
    agent: &Agent,
    history: History,
    tool_name_to_node: HashMap<String, Arc<InternalNode>>,
) -> flow_like_types::Result<AgentExecutionResult> {
    // Create a no-op stream state that doesn't emit chunks
    let stream_state = NoOpStreamState {};

    // Call the streaming version with the no-op handler
    execute_agent_streaming(context, agent, history, tool_name_to_node, &stream_state).await
}

/// No-op stream handler for non-streaming agent execution
#[cfg(feature = "execute")]
struct NoOpStreamState {}

#[cfg(feature = "execute")]
#[async_trait]
impl StreamHandler for NoOpStreamState {
    async fn emit_chunk(
        &self,
        _context: &mut ExecutionContext,
        _chunk: &ResponseChunk,
    ) -> flow_like_types::Result<()> {
        // Do nothing - this is for non-streaming mode
        Ok(())
    }

    async fn finalize(&self, _context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        // Do nothing - this is for non-streaming mode
        Ok(())
    }
}

/// Stream handler for emitting chunks during agent execution
#[cfg(feature = "execute")]
pub struct AgentStreamState {
    parent_node_id: String,
    chunk_pin_available: bool,
    on_stream_exists: bool,
    connected_nodes: Option<Arc<DashMap<String, Arc<Mutex<ExecutionContext>>>>>,
}

#[cfg(feature = "execute")]
impl AgentStreamState {
    pub async fn new(context: &mut ExecutionContext) -> flow_like_types::Result<Self> {
        let parent_node_id = context.node.node.lock().await.id.clone();
        let chunk_pin_available = context.get_pin_by_name("chunk").await.is_ok();

        let mut on_stream_exists = false;
        let mut connected_nodes = None;

        if let Ok(on_stream_pin) = context.get_pin_by_name("on_stream").await {
            on_stream_exists = true;
            context.activate_exec_pin_ref(&on_stream_pin).await?;
            let connected = on_stream_pin.get_connected_nodes();
            if !connected.is_empty() {
                let map = Arc::new(DashMap::new());
                for node in connected {
                    let sub_context = context.create_sub_context(&node).await;
                    map.insert(
                        node.node.lock().await.id.clone(),
                        Arc::new(Mutex::new(sub_context)),
                    );
                }
                connected_nodes = Some(map);
            }
        }

        Ok(Self {
            parent_node_id,
            chunk_pin_available,
            on_stream_exists,
            connected_nodes,
        })
    }
}

#[cfg(feature = "execute")]
#[async_trait]
impl StreamHandler for AgentStreamState {
    async fn emit_chunk(
        &self,
        context: &mut ExecutionContext,
        chunk: &ResponseChunk,
    ) -> flow_like_types::Result<()> {
        if !self.chunk_pin_available && self.connected_nodes.is_none() {
            return Ok(());
        }

        if self.chunk_pin_available {
            context
                .set_pin_value("chunk", json::json!(chunk.clone()))
                .await?;
        }

        if let Some(nodes) = &self.connected_nodes {
            let mut recursion_guard = AHashSet::new();
            recursion_guard.insert(self.parent_node_id.clone());

            for entry in nodes.iter() {
                let (id, sub_context) = entry.pair();
                let mut sub_context = sub_context.lock().await;
                let mut guard = Some(recursion_guard.clone());
                let run = InternalNode::trigger(&mut sub_context, &mut guard, true).await;
                sub_context.end_trace();
                if let Err(err) = run {
                    context.log_message(
                        &format!("Stream-connected node {} failed: {:?}", id, err),
                        LogLevel::Error,
                    );
                }
            }
        }

        Ok(())
    }

    async fn finalize(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        if self.on_stream_exists {
            context.deactivate_exec_pin("on_stream").await?;
        }

        if let Some(nodes) = &self.connected_nodes {
            for entry in nodes.iter() {
                let (_, sub_context) = entry.pair();
                let mut sub_context = sub_context.lock().await;
                sub_context.end_trace();
                context.push_sub_context(&mut sub_context);
            }
        }

        Ok(())
    }
}

/// Execute an agent with streaming support.
/// Emits chunks through the provided stream state as the agent generates responses.
#[cfg(feature = "execute")]
pub async fn execute_agent_streaming(
    context: &mut ExecutionContext,
    agent: &Agent,
    history: History,
    tool_name_to_node: HashMap<String, Arc<InternalNode>>,
    stream_state: &dyn StreamHandler,
) -> flow_like_types::Result<AgentExecutionResult> {
    let model_display_name = agent
        .model
        .meta
        .get("name")
        .map(|meta| meta.name.clone())
        .unwrap_or_else(|| agent.model.id.clone());

    let system_prompt = agent
        .get_system_prompt()
        .unwrap_or_else(|| "You are a helpful assistant with access to tools.".to_string());

    let agent_builder = agent
        .model
        .agent(context, &Some(history.clone()))
        .await?
        .preamble(&system_prompt);
    let mut tool_servers: Vec<(Vec<rmcp::model::Tool>, rmcp::service::ServerSink)> = Vec::new();
    let mut mcp_tool_clients: HashMap<String, rmcp::service::ServerSink> = HashMap::new();
    let mut _mcp_clients = Vec::new();

    let client_info = ClientInfo {
        protocol_version: Default::default(),
        capabilities: ClientCapabilities::default(),
        client_info: Implementation {
            name: "Flow-Like".to_string(),
            version: "alpha".to_string(),
            title: None,
            icons: None,
            website_url: Some("https://flow-like.com".to_string()),
        },
    };

    for mcp_config in &agent.mcp_servers {
        let transport =
            rmcp::transport::StreamableHttpClientTransport::from_uri(mcp_config.uri.as_str());
        let client = match client_info.clone().serve(transport).await {
            Ok(c) => c,
            Err(e) => {
                let error = format!("Failed to connect to MCP server {}: {}", mcp_config.uri, e);
                context.log_message(&error, LogLevel::Error);
                continue;
            }
        };

        // Fetch all tools with pagination support
        let mut all_tools = Vec::new();
        let mut cursor: Option<PaginatedRequestParam> = None;

        loop {
            let list_result = client.list_tools(cursor.clone()).await;

            let response = match list_result {
                Ok(r) => r,
                Err(e) => {
                    let error = format!(
                        "Failed to fetch tools from MCP server {}: {}",
                        mcp_config.uri, e
                    );
                    context.log_message(&error, LogLevel::Error);
                    break;
                }
            };

            all_tools.extend(response.tools);

            // Check if there are more pages
            if let Some(next_cursor) = response.next_cursor {
                cursor = Some(PaginatedRequestParam {
                    cursor: Some(next_cursor),
                });
            } else {
                break;
            }
        }

        if all_tools.is_empty() {
            context.log_message(
                &format!("No tools available from MCP server {}", mcp_config.uri),
                LogLevel::Warn,
            );
            continue;
        }

        let filtered_tools = if let Some(filter) = &mcp_config.tool_filter {
            all_tools
                .into_iter()
                .filter(|t| filter.contains(&*t.name))
                .collect()
        } else {
            all_tools
        };

        if filtered_tools.is_empty() {
            context.log_message(
                &format!(
                    "No matching tools after filtering for MCP server {}",
                    mcp_config.uri
                ),
                LogLevel::Warn,
            );
            continue;
        }

        let peer = client.peer().to_owned();

        for tool in &filtered_tools {
            let tool_name = tool.name.to_string();
            if mcp_tool_clients
                .insert(tool_name.clone(), peer.clone())
                .is_some()
            {
                context.log_message(
                    &format!(
                        "Duplicate MCP tool name '{}' detected; using the latest registration",
                        tool_name
                    ),
                    LogLevel::Warn,
                );
            }
        }

        tool_servers.push((filtered_tools, peer));
        _mcp_clients.push(client);
    }

    let mut tool_iter = tool_servers.into_iter();
    let rig_agent = if let Some((tools, peer)) = tool_iter.next() {
        let mut simple_builder = agent_builder.rmcp_tools(tools, peer);
        for (tools, peer) in tool_iter {
            simple_builder = simple_builder.rmcp_tools(tools, peer);
        }
        // Add ThinkTool if thinking is enabled for the agent
        if agent.thinking_enabled {
            simple_builder = simple_builder.tool(ThinkTool);
        }
        simple_builder.build()
    } else {
        // No MCP tools, check if we need to add ThinkTool
        if agent.thinking_enabled {
            agent_builder.tool(ThinkTool).build()
        } else {
            agent_builder.build()
        }
    };

    // Build tool definitions from both:
    // 1. Explicit tools stored in agent.tools
    // 2. Function references that need to be converted to tools
    let mut tool_definitions: Vec<ToolDefinition> = agent
        .tools
        .iter()
        .map(|tool| {
            let parameters =
                json::to_value(&tool.function.parameters).unwrap_or_else(|_| json::json!({}));
            ToolDefinition {
                name: tool.function.name.clone(),
                description: tool.function.description.clone().unwrap_or_default(),
                parameters,
            }
        })
        .collect();

    // Generate tool definitions from function references
    for internal_node in tool_name_to_node.values() {
        let tool = generate_tool_from_function(internal_node).await?;
        let parameters =
            json::to_value(&tool.function.parameters).unwrap_or_else(|_| json::json!({}));
        tool_definitions.push(ToolDefinition {
            name: tool.function.name.clone(),
            description: tool.function.description.clone().unwrap_or_default(),
            parameters,
        });
    }

    // Deduplicate tools by name, keeping the first occurrence
    let mut seen_tool_names = std::collections::HashSet::new();
    tool_definitions.retain(|tool| seen_tool_names.insert(tool.name.clone()));

    let (prompt, history_msgs) = history
        .extract_prompt_and_history()
        .map_err(|e| anyhow!("Failed to convert history: {e}"))?;

    // Filter out tool-related messages to start fresh
    // We need to ensure tool results always follow their corresponding tool calls
    // The safest approach is to remove all tool-related messages from input history
    let mut current_history: Vec<rig::message::Message> = Vec::new();
    let mut pending_tool_call_ids: std::collections::HashSet<String> =
        std::collections::HashSet::new();

    for msg in history_msgs {
        match &msg {
            rig::message::Message::User { content } => {
                // Check if this is a tool result
                let has_tool_result = content
                    .iter()
                    .any(|c| matches!(c, rig::message::UserContent::ToolResult(_)));
                if has_tool_result {
                    // Only include if we have a pending tool call for it
                    for c in content.iter() {
                        if let rig::message::UserContent::ToolResult(tr) = c {
                            if pending_tool_call_ids.remove(&tr.id) {
                                current_history.push(msg.clone());
                                break;
                            }
                        }
                    }
                } else {
                    current_history.push(msg);
                }
            }
            rig::message::Message::Assistant { content, .. } => {
                // Check if this message has tool calls
                let mut has_tool_calls = false;
                for c in content.iter() {
                    if let rig::message::AssistantContent::ToolCall(tc) = c {
                        has_tool_calls = true;
                        pending_tool_call_ids.insert(tc.id.clone());
                    }
                }
                if has_tool_calls {
                    current_history.push(msg);
                } else {
                    current_history.push(msg);
                }
            }
        }
    }

    // Remove any trailing assistant messages with tool calls that don't have results
    // (iterate backwards and remove until we find a non-tool-call message)
    while let Some(last) = current_history.last() {
        if let rig::message::Message::Assistant { content, .. } = last {
            let has_tool_calls = content
                .iter()
                .any(|c| matches!(c, rig::message::AssistantContent::ToolCall(_)));
            if has_tool_calls {
                current_history.pop();
                continue;
            }
        }
        break;
    }

    // Apply initial context management if infinite context mode is enabled
    let max_context_tokens = agent
        .max_context_tokens
        .unwrap_or(DEFAULT_MAX_CONTEXT_TOKENS);
    if agent.infinite_context {
        let (managed, count) =
            manage_context_budget(context, agent, current_history, max_context_tokens).await?;
        current_history = managed;
        if count > 0 {
            let mode_name = match agent.context_management_mode {
                ContextManagementMode::Summarize => "summarized",
                ContextManagementMode::Truncate => "truncated",
            };
            context.log_message(
                &format!(
                    "Infinite context: {} {} messages from initial history",
                    mode_name, count
                ),
                LogLevel::Debug,
            );
        }
    }

    let mut full_history = history.clone();
    let mut iteration = 0;

    loop {
        if iteration >= agent.max_iterations {
            return Err(anyhow!(
                "Max recursion limit ({}) reached",
                agent.max_iterations
            ));
        }

        let mut request = rig_agent
            .completion(prompt.clone(), current_history.clone())
            .await
            .map_err(|e| anyhow!("Agent completion failed: {}", e))?;

        if !tool_definitions.is_empty() {
            request = request.tools(tool_definitions.clone());
        }

        let mut stream = request
            .stream()
            .await
            .map_err(|e| anyhow!("Failed to start completion stream: {}", e))?;

        let mut response_contents: Vec<AssistantContent> = Vec::new();
        let mut final_usage: Option<RigUsage> = None;
        let mut response_obj = Response::new();
        response_obj.model = Some(model_display_name.clone());

        // Track tool call deltas to accumulate them into complete tool calls
        // Key: tool call ID, Value: (name, arguments)
        let mut tool_call_deltas: HashMap<String, (String, String)> = HashMap::new();
        // Track IDs of complete tool calls to avoid duplicates
        let mut complete_tool_call_ids: std::collections::HashSet<String> =
            std::collections::HashSet::new();

        while let Some(item) = stream.next().await {
            let content = item.map_err(|e| anyhow!("Streaming error: {}", e))?;

            match content {
                StreamedAssistantContent::Text(text) => {
                    let chunk = ResponseChunk::from_text(&text.text, &model_display_name);
                    response_obj.push_chunk(chunk.clone());
                    stream_state.emit_chunk(context, &chunk).await?;
                    response_contents.push(AssistantContent::Text(text));
                }
                StreamedAssistantContent::ToolCall(tool_call) => {
                    let chunk = ResponseChunk::from_tool_call(&tool_call, &model_display_name);
                    response_obj.push_chunk(chunk.clone());
                    stream_state.emit_chunk(context, &chunk).await?;
                    // Track this ID so we don't duplicate from deltas
                    complete_tool_call_ids.insert(tool_call.id.clone());
                    response_contents.push(AssistantContent::ToolCall(tool_call));
                }
                StreamedAssistantContent::ToolCallDelta { id, content } => {
                    let entry = tool_call_deltas
                        .entry(id.clone())
                        .or_insert((String::new(), String::new()));
                    let delta_str = match &content {
                        rig::streaming::ToolCallDeltaContent::Name(name) => {
                            entry.0.push_str(name);
                            name.clone()
                        }
                        rig::streaming::ToolCallDeltaContent::Delta(delta) => {
                            entry.1.push_str(delta);
                            delta.clone()
                        }
                    };
                    let chunk =
                        ResponseChunk::from_tool_call_delta(&id, &delta_str, &model_display_name);
                    response_obj.push_chunk(chunk.clone());
                    stream_state.emit_chunk(context, &chunk).await?;
                }
                StreamedAssistantContent::Reasoning(reasoning) => {
                    let reasoning_text = reasoning.reasoning.join("\n");
                    let chunk = ResponseChunk::from_reasoning(&reasoning_text, &model_display_name);
                    response_obj.push_chunk(chunk.clone());
                    stream_state.emit_chunk(context, &chunk).await?;
                }
                StreamedAssistantContent::ReasoningDelta { reasoning, .. } => {
                    let chunk = ResponseChunk::from_reasoning(&reasoning, &model_display_name);
                    response_obj.push_chunk(chunk.clone());
                    stream_state.emit_chunk(context, &chunk).await?;
                }
                StreamedAssistantContent::Final(final_resp) => {
                    final_usage = final_resp.usage;
                }
            }
        }

        let finish_chunk = ResponseChunk::finish(&model_display_name, final_usage.as_ref());
        response_obj.push_chunk(finish_chunk.clone());
        stream_state.emit_chunk(context, &finish_chunk).await?;

        if let Some(usage) = final_usage {
            response_obj.usage = ResponseUsage::from_rig(usage);
        }

        // Convert accumulated tool call deltas into complete ToolCall entries
        // Skip any that we already have as complete tool calls
        for (id, (name, arguments)) in tool_call_deltas {
            if !name.is_empty() && !complete_tool_call_ids.contains(&id) {
                let tool_call = RigToolCall {
                    id: id.clone(),
                    call_id: None,
                    function: rig::message::ToolFunction {
                        name,
                        arguments: json::from_str(&arguments).unwrap_or(json::json!({})),
                    },
                    signature: None,
                    additional_params: None,
                };
                response_contents.push(AssistantContent::ToolCall(tool_call));
            }
        }

        let assistant_msg = rig::message::Message::Assistant {
            id: None,
            content: OneOrMany::many(response_contents.clone()).unwrap_or_else(|_| {
                OneOrMany::one(AssistantContent::Text(rig::message::Text {
                    text: String::new(),
                }))
            }),
        };

        let mut tool_calls_found = false;
        let mut tool_results: Vec<(String, String, Value)> = Vec::new();

        for content in response_contents.iter() {
            if let AssistantContent::ToolCall(RigToolCall {
                id,
                call_id: _,
                function:
                    rig::message::ToolFunction {
                        name, arguments, ..
                    },
                ..
            }) = content
            {
                tool_calls_found = true;

                let tool_output = if let Some(referenced_node) = tool_name_to_node.get(name) {
                    let result = execute_tool_call(context, referenced_node, name, arguments).await;

                    match result {
                        Ok(value) => value,
                        Err(error) => json::json!(format!("Error: {:?}", error)),
                    }
                } else if let Some(mcp_peer) = mcp_tool_clients.get(name) {
                    context.log_message(
                        &format!("Calling MCP tool '{}' with arguments {}", name, arguments),
                        LogLevel::Debug,
                    );

                    let args_map = arguments.as_object().cloned();
                    match mcp_peer
                        .call_tool(CallToolRequestParam {
                            name: name.clone().into(),
                            arguments: args_map,
                            task: None,
                        })
                        .await
                    {
                        Ok(result) => {
                            context.log_message(
                                &format!(
                                    "MCP tool '{}' returned successfully with result {:?}",
                                    name, result
                                ),
                                LogLevel::Debug,
                            );
                            json::to_value(result)
                                .unwrap_or_else(|_| json::json!({"message": "Tool executed"}))
                        }
                        Err(error) => {
                            context.log_message(
                                &format!("MCP tool '{}' call failed: {}", name, error),
                                LogLevel::Error,
                            );
                            json::json!({"error": format!("{}", error)})
                        }
                    }
                } else if name == "think" && agent.thinking_enabled {
                    // Handle the built-in think tool - extract the thought and wrap in <think> tags
                    let thought = arguments
                        .get("thought")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    context.log_message(
                        &format!("Think tool called with thought: {}", thought),
                        LogLevel::Debug,
                    );
                    // Return the thought wrapped in <think> tags for frontend rendering
                    json::json!(format!("<think>{}</think>", thought))
                } else {
                    return Err(anyhow!(
                        "Tool '{}' not found in referenced functions or MCP servers",
                        name
                    ));
                };

                tool_results.push((id.clone(), name.clone(), tool_output));
            }
        }

        if !tool_calls_found {
            let final_response = response_obj.content().unwrap_or_default();
            let final_assistant_msg = HistoryMessage {
                role: Role::Assistant,
                content: MessageContent::String(final_response.clone()),
                name: None,
                tool_call_id: None,
                tool_calls: None,
                annotations: None,
            };
            full_history.push_message(final_assistant_msg);

            return Ok(AgentExecutionResult {
                response: response_obj,
                history: full_history,
            });
        }

        let assistant_clone = assistant_msg.clone();
        current_history.push(assistant_clone.clone());

        let assistant_history_msg: HistoryMessage = assistant_clone.into();
        full_history.push_message(assistant_history_msg);

        use rig::message::{ToolResult as RigToolResult, ToolResultContent, UserContent};

        // Collect all tool results into a single User message
        // This is required for Gemini API which expects tool results to immediately follow
        // the assistant's tool call message in a single message
        let mut tool_result_contents: Vec<UserContent> = Vec::new();

        for (tool_id, tool_name, tool_output) in &tool_results {
            let tool_result_str = match tool_output.as_str() {
                Some(s) => s.to_string(),
                None => json::to_string(tool_output).unwrap_or_default(),
            };

            tool_result_contents.push(UserContent::ToolResult(RigToolResult {
                id: tool_id.clone(),
                call_id: None,
                content: OneOrMany::one(ToolResultContent::text(tool_result_str.clone())),
            }));

            let tool_msg = HistoryMessage {
                role: Role::Tool,
                content: MessageContent::Contents(vec![Content::Text {
                    content_type: ContentType::Text,
                    text: tool_result_str,
                }]),
                name: Some(tool_name.clone()),
                tool_call_id: Some(tool_id.clone()),
                tool_calls: None,
                annotations: None,
            };
            full_history.push_message(tool_msg);
        }

        // Add all tool results as a single User message
        if !tool_result_contents.is_empty() {
            let combined_tool_results = if tool_result_contents.len() == 1 {
                OneOrMany::one(tool_result_contents.into_iter().next().unwrap())
            } else {
                // For multiple tool results, create a Many variant
                // This should never fail since we already checked len > 1
                OneOrMany::many(tool_result_contents)
                    .expect("tool_result_contents should have at least 2 elements")
            };

            let tool_result_msg = rig::message::Message::User {
                content: combined_tool_results,
            };
            current_history.push(tool_result_msg);
        }

        // Apply context management after adding tool results if infinite context is enabled
        if agent.infinite_context {
            let (managed, count) =
                manage_context_budget(context, agent, current_history, max_context_tokens).await?;
            current_history = managed;
            if count > 0 {
                let mode_name = match agent.context_management_mode {
                    ContextManagementMode::Summarize => "summarized",
                    ContextManagementMode::Truncate => "truncated",
                };
                context.log_message(
                    &format!(
                        "Infinite context: {} {} messages at iteration {}",
                        mode_name, count, iteration
                    ),
                    LogLevel::Debug,
                );
            }
        }

        iteration += 1;
    }
}
