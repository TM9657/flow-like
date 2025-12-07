use crate::ai::generative::agent::Agent;
/// # Agent Execution Helpers
/// This module contains reusable logic for executing agents with tools and streaming.
/// Extracted from simple.rs to be shared across multiple agent nodes.
use ahash::AHashSet;
use flow_like::flow::{
    execution::{LogLevel, context::ExecutionContext, internal_node::InternalNode},
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
use futures::StreamExt;
use rig::OneOrMany;
use rig::completion::{Completion, ToolDefinition, Usage as RigUsage};
use rig::message::{AssistantContent, ToolCall as RigToolCall};
use rig::streaming::StreamedAssistantContent;
use rmcp::{
    ServiceExt,
    model::{
        CallToolRequestParam, ClientCapabilities, ClientInfo, Implementation, PaginatedRequestParam,
    },
};
use std::{collections::HashMap, sync::Arc};
/// Generate OpenAI function call schema from a referenced function node.
/// Returns a Tool definition with function name, description, and parameter schema.
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
pub struct AgentExecutionResult {
    pub response: Response,
    pub history: History,
}

/// Trait for handling stream emissions during agent execution
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
struct NoOpStreamState {}

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
pub struct AgentStreamState {
    parent_node_id: String,
    chunk_pin_available: bool,
    on_stream_exists: bool,
    connected_nodes: Option<Arc<DashMap<String, Arc<Mutex<ExecutionContext>>>>>,
}

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
        simple_builder.build()
    } else {
        agent_builder.build()
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

    let (prompt, history_msgs) = history
        .extract_prompt_and_history()
        .map_err(|e| anyhow!("Failed to convert history: {e}"))?;

    let mut current_history: Vec<rig::message::Message> = history_msgs
        .into_iter()
        .filter(|msg| match msg {
            rig::message::Message::User { content } => !content
                .iter()
                .any(|c| matches!(c, rig::message::UserContent::ToolResult(_))),
            _ => true,
        })
        .collect();

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
                    response_contents.push(AssistantContent::ToolCall(tool_call));
                }
                StreamedAssistantContent::ToolCallDelta { id, delta } => {
                    let chunk =
                        ResponseChunk::from_tool_call_delta(&id, &delta, &model_display_name);
                    response_obj.push_chunk(chunk.clone());
                    stream_state.emit_chunk(context, &chunk).await?;
                }
                StreamedAssistantContent::Reasoning(reasoning) => {
                    let reasoning_text = reasoning.reasoning.join("\n");
                    let chunk = ResponseChunk::from_reasoning(&reasoning_text, &model_display_name);
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

        for (tool_id, tool_name, tool_output) in &tool_results {
            let tool_result_str = match tool_output.as_str() {
                Some(s) => s.to_string(),
                None => json::to_string(tool_output).unwrap_or_default(),
            };

            let tool_result_msg = rig::message::Message::User {
                content: OneOrMany::one(UserContent::ToolResult(RigToolResult {
                    id: tool_id.clone(),
                    call_id: None,
                    content: OneOrMany::one(ToolResultContent::text(tool_result_str.clone())),
                })),
            };
            current_history.push(tool_result_msg);

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

        iteration += 1;
    }
}
