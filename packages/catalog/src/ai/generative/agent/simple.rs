/// # Simple Agent Node
/// This is an LLM-controlled while loop over an arbitrary number of flow-leafes with back-propagation of leaf outputs into the agent.
/// Uses Rig's agent system with dynamic tools for executing Flow-Like subcontexts.
/// Recursive agent calls until no more tool calls are made or recursion limit hit.
/// Effectively, this node allows the LLM to control it's own execution until further human input required.
use flow_like::{
    bit::Bit,
    flow::{
        board::Board,
        execution::{LogLevel, context::ExecutionContext, internal_node::InternalNode},
        node::{Node, NodeLogic},
        pin::{PinOptions, PinType},
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_model_provider::{
    history::{History, Tool},
    response::Response,
};

use flow_like_types::{Value, anyhow, async_trait, json};
use rig::completion::{Completion, ToolDefinition};
use rig::message::{AssistantContent, ToolCall as RigToolCall};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

#[crate::register_node]
#[derive(Default)]
pub struct SimpleAgentNode {}

impl SimpleAgentNode {
    pub fn new() -> Self {
        SimpleAgentNode {}
    }
}

#[async_trait]
impl NodeLogic for SimpleAgentNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "simple_agent",
            "Simple Agent",
            "Simple Agent Node with Tool Calls",
            "AI/Agents",
        );
        node.add_icon("/flow/icons/for-each.svg");
        node.set_can_reference_fns(true);

        node.add_input_pin("exec_in", "Input", "Trigger Pin", VariableType::Execution);

        node.add_input_pin("model", "Model", "Model", VariableType::Struct)
            .set_schema::<Bit>()
            .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_input_pin("history", "History", "Chat History", VariableType::Struct)
            .set_schema::<History>()
            .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_input_pin(
            "tools",
            "Tools",
            "JSON or OpenAI Function Definitions",
            VariableType::String,
        )
        .set_default_value(Some(json::json!("[]")));

        node.add_input_pin(
            "max_iter",
            "Iter",
            "Maximum Number of Internal Agent Iterations (Recursion Limit)",
            VariableType::Integer,
        )
        .set_default_value(Some(json::json!(15)));

        node.add_output_pin("exec_done", "Done", "Done Pin", VariableType::Execution);

        node.add_output_pin(
            "response",
            "Response",
            "Final Response (Agent decides to stop execution)",
            VariableType::Struct,
        )
        .set_schema::<Response>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_output_pin(
            "history_out",
            "History Out",
            "Updated History with all agent interactions",
            VariableType::Struct,
        )
        .set_schema::<History>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_output_pin(
            "tool_call_id",
            "Tool Call Id",
            "Tool Call Identifier",
            VariableType::String,
        );

        node.add_output_pin(
            "tool_call_args",
            "Tool Call Args",
            "Tool Call Arguments",
            VariableType::Struct,
        );

        node.set_long_running(true);

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_done").await?;

        let max_iterations: u64 = context.evaluate_pin("max_iter").await?;
        let model_bit = context.evaluate_pin::<Bit>("model").await?;
        let tools_str: String = context.evaluate_pin("tools").await?;
        let history = context.evaluate_pin::<History>("history").await?;

        let tools: Vec<Tool> =
            json::from_str(&tools_str).map_err(|err| anyhow!("Failed to parse tools: {err:?}"))?;

        for tool in &tools {
            context.deactivate_exec_pin(&tool.function.name).await?;
        }

        if let Some(meta) = model_bit.meta.get("en") {
            context.log_message(&format!("Loading model {:?}", meta.name), LogLevel::Debug);
        }

        let system_prompt = history
            .get_system_prompt()
            .unwrap_or_else(|| "You are a helpful assistant with access to tools.".to_string());

        let agent_builder = model_bit.agent(context, &Some(history.clone())).await?;
        let agent = agent_builder.preamble(&system_prompt).build();

        let tool_definitions: Vec<ToolDefinition> = tools
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

        let (prompt, history_msgs) = history
            .extract_prompt_and_history()
            .map_err(|e| anyhow!("Failed to convert history: {e}"))?;

        context.log_message(
            &format!("Initial history_msgs count: {}", history_msgs.len()),
            LogLevel::Debug,
        );

        // Use multi-turn loop with completion client
        // Filter out tool result messages from previous agent runs to avoid confusing Rig
        let mut current_history: Vec<rig::message::Message> = history_msgs
            .into_iter()
            .filter(|msg| {
                // Keep all messages except User messages with ToolResult content
                // These are from previous tool executions and should not be included
                match msg {
                    rig::message::Message::User { content } => {
                        // Check if any content is a ToolResult
                        let has_tool_result = content
                            .iter()
                            .any(|c| matches!(c, rig::message::UserContent::ToolResult(_)));
                        !has_tool_result
                    }
                    _ => true,
                }
            })
            .collect();

        context.log_message(
            &format!(
                "After filtering, current_history count: {}",
                current_history.len()
            ),
            LogLevel::Debug,
        );

        let mut full_history = history.clone(); // Track full history including tool results
        let mut iteration = 0;

        loop {
            if iteration >= max_iterations {
                return Err(anyhow!("Max recursion limit ({}) reached", max_iterations));
            }

            context.log_message(
                &format!(
                    "[agent iter {}] Starting iteration (current_history: {}, full_history: {})",
                    iteration,
                    current_history.len(),
                    full_history.messages.len()
                ),
                LogLevel::Debug,
            );

            // Make completion request with tools
            let mut request = agent
                .completion(prompt.clone(), current_history.clone())
                .await
                .map_err(|e| anyhow!("Agent completion failed: {}", e))?;

            // Add tool definitions to request if we have tools
            if !tool_definitions.is_empty() {
                context.log_message(
                    &format!(
                        "Adding {} tool definitions to request",
                        tool_definitions.len()
                    ),
                    LogLevel::Debug,
                );
                request = request.tools(tool_definitions.clone());
            }

            // Send request
            let response = request
                .send()
                .await
                .map_err(|e| anyhow!("Failed to send completion request: {}", e))?;

            context.log_message(
                &format!("Received response with {} choices", response.choice.len()),
                LogLevel::Debug,
            );

            // Check for tool calls in response
            let mut tool_calls_found = false;
            let mut tool_results: Vec<(String, String, Value)> = Vec::new();

            for content in response.choice.iter() {
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
                    context.log_message(
                        &format!(
                            "[agent iter {}] Found tool call: {} (id: {})",
                            iteration, name, id
                        ),
                        LogLevel::Debug,
                    );

                    // Set tool call outputs on pins
                    let tool_call_id_pin = context.get_pin_by_name("tool_call_id").await?;
                    let tool_call_args_pin = context.get_pin_by_name("tool_call_args").await?;

                    tool_call_id_pin
                        .lock()
                        .await
                        .set_value(json::json!(id))
                        .await;
                    tool_call_args_pin
                        .lock()
                        .await
                        .set_value(arguments.clone())
                        .await;

                    // Activate the specific tool exec pin
                    context.activate_exec_pin(name.as_str()).await?;

                    // Execute tool subcontext
                    let tool_exec_pin = context.get_pin_by_name(name.as_str()).await?;
                    let tool_flow = tool_exec_pin.lock().await.get_connected_nodes().await;

                    context.log_message(
                        &format!("Tool {} has {} connected nodes", name, tool_flow.len()),
                        LogLevel::Debug,
                    );

                    for (node_idx, node) in tool_flow.iter().enumerate() {
                        context.log_message(
                            &format!(
                                "Executing tool node {}/{} for tool {}",
                                node_idx + 1,
                                tool_flow.len(),
                                name
                            ),
                            LogLevel::Debug,
                        );

                        let mut sub_context = context.create_sub_context(node).await;
                        let run = InternalNode::trigger(&mut sub_context, &mut None, true).await;

                        // CRITICAL: Capture result BEFORE end_trace and push_sub_context
                        // push_sub_context only copies traces, not the result field!
                        let captured_result = sub_context.result.clone();

                        sub_context.end_trace();
                        context.push_sub_context(&mut sub_context);

                        // Prepare tool output message for history
                        let tool_output_str = match run {
                            Ok(_) => {
                                // Use captured result
                                if let Some(ref result) = captured_result {
                                    let result_str = json::to_string(&result)
                                        .unwrap_or_else(|_| result.to_string());
                                    context.log_message(
                                        &format!(
                                            "Tool {} returned result ({} chars)",
                                            name,
                                            result_str.len()
                                        ),
                                        LogLevel::Debug,
                                    );
                                    result_str
                                } else {
                                    context.log_message(
                                        &format!("Tool {} executed successfully (no result)", name),
                                        LogLevel::Debug,
                                    );
                                    "Tool executed successfully".to_string()
                                }
                            }
                            Err(error) => {
                                context.log_message(
                                    &format!("Tool {} execution FAILED: {:?}", name, error),
                                    LogLevel::Error,
                                );
                                format!("Error: {:?}", error)
                            }
                        };

                        tool_results.push((id.clone(), name.clone(), json::json!(tool_output_str)));
                    }

                    // Deactivate tool exec pin
                    context.deactivate_exec_pin(name.as_str()).await?;
                }
            }

            context.log_message(
                &format!("Tool results collected: {}", tool_results.len()),
                LogLevel::Debug,
            );

            // Log all tool results
            for (idx, (tool_id, tool_name, tool_output)) in tool_results.iter().enumerate() {
                context.log_message(
                    &format!("Tool result {}: name={}, id={}", idx, tool_name, tool_id),
                    LogLevel::Debug,
                );
            }

            // If no tool calls, we're done
            if !tool_calls_found {
                context.log_message(
                    &format!("[agent iter {}] No more tool calls, finishing", iteration),
                    LogLevel::Debug,
                );

                // Extract final text response
                let final_response = response
                    .choice
                    .iter()
                    .find_map(|c| match c {
                        AssistantContent::Text(text) => Some(text.text.clone()),
                        _ => None,
                    })
                    .unwrap_or_else(String::new);

                context.log_message(
                    &format!("Final response extracted: {} chars", final_response.len()),
                    LogLevel::Debug,
                );

                // Create Response object from the final response
                use rig::OneOrMany;
                let content = OneOrMany::one(AssistantContent::Text(rig::message::Text {
                    text: final_response.clone(),
                }));

                let response_obj = Response::from_rig_message(rig::message::Message::Assistant {
                    id: None,
                    content,
                })
                .map_err(|e| anyhow!("Failed to create response: {}", e))?;

                // Add final assistant response to full history
                use flow_like_model_provider::history::{HistoryMessage, MessageContent, Role};
                let final_assistant_msg = HistoryMessage {
                    role: Role::Assistant,
                    content: MessageContent::String(final_response),
                    name: None,
                    tool_call_id: None,
                    tool_calls: None,
                    annotations: None,
                };
                full_history.push_message(final_assistant_msg);

                // Try to set history_out if it exists (may not exist in older node instances)
                if context.get_pin_by_name("history_out").await.is_ok() {
                    context
                        .set_pin_value("history_out", json::json!(full_history))
                        .await?;
                }

                context
                    .set_pin_value("response", json::json!(response_obj))
                    .await?;

                context.activate_exec_pin("exec_done").await?;

                context.log_message("Agent execution complete", LogLevel::Debug);
                return Ok(());
            }

            // Prepare history for next iteration by appending assistant message
            let assistant_msg = rig::message::Message::Assistant {
                id: None,
                content: response.choice.clone(),
            };
            current_history.push(assistant_msg.clone());

            // Add assistant message with tool calls to full history
            use flow_like_model_provider::history::{
                Content, ContentType, HistoryMessage, MessageContent, Role,
            };
            let assistant_history_msg: HistoryMessage = assistant_msg.into();
            full_history.push_message(assistant_history_msg);

            context.log_message(
                &format!("Adding {} tool results to histories", tool_results.len()),
                LogLevel::Debug,
            );
            // Add tool results directly to current_history as Rig UserContent::ToolResult messages
            // This is what Rig expects for multi-turn tool execution
            use rig::OneOrMany;
            use rig::message::{ToolResult as RigToolResult, ToolResultContent, UserContent};

            for (tool_id, tool_name, tool_output) in &tool_results {
                let tool_result_str = match tool_output.as_str() {
                    Some(s) => s.to_string(),
                    None => json::to_string(tool_output).unwrap_or_default(),
                };

                context.log_message(
                    &format!(
                        "Adding tool result to history: {} (id: {}) -> {} chars",
                        tool_name,
                        tool_id,
                        tool_result_str.len()
                    ),
                    LogLevel::Debug,
                );

                // Create Rig-native tool result message
                let tool_result_msg = rig::message::Message::User {
                    content: OneOrMany::one(UserContent::ToolResult(RigToolResult {
                        id: tool_id.clone(),
                        call_id: None,
                        content: OneOrMany::one(ToolResultContent::text(tool_result_str.clone())),
                    })),
                };
                current_history.push(tool_result_msg);

                // Also add to full_history for output tracking
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
            context.log_message(
                &format!("Iteration {} complete, continuing loop", iteration - 1),
                LogLevel::Debug,
            );
        }
    }

    async fn on_update(&self, node: &mut Node, _board: Arc<Board>) {
        let current_tool_exec_pins: Vec<_> = node
            .pins
            .values()
            .filter(|p| {
                p.pin_type == PinType::Output
                    && (p.name != "exec_done"
                        && p.name != "response"
                        && p.name != "tool_call_id"
                        && p.name != "tool_call_args") // p.description == "Tool Exec" doesn't seem to work as filter cond
            })
            .collect();

        let tools_str: String = node
            .get_pin_by_name("tools")
            .and_then(|pin| pin.default_value.clone())
            .and_then(|bytes| flow_like_types::json::from_slice::<Value>(&bytes).ok())
            .and_then(|json| json.as_str().map(ToOwned::to_owned))
            .unwrap_or_default();

        let mut current_tool_exec_refs = current_tool_exec_pins
            .iter()
            .map(|p| (p.name.clone(), *p))
            .collect::<HashMap<_, _>>();

        let update_tools: Vec<Tool> = match json::from_str(&tools_str) {
            Ok(tools) => tools,
            Err(err) => {
                node.error = Some(format!("Failed to parse tools: {err:?}").to_string());
                return;
            }
        };

        let mut all_tool_exec_refs = HashSet::new();
        let mut missing_tool_exec_refs = HashSet::new();

        for update_tool in update_tools {
            all_tool_exec_refs.insert(update_tool.function.name.clone());
            if current_tool_exec_refs
                .remove(&update_tool.function.name)
                .is_none()
            {
                missing_tool_exec_refs.insert(update_tool.function.name.clone());
            }
        }

        let ids_to_remove = current_tool_exec_refs
            .values()
            .map(|p| p.id.clone())
            .collect::<Vec<_>>();
        ids_to_remove.iter().for_each(|id| {
            node.pins.remove(id);
        });

        for missing_tool_ref in missing_tool_exec_refs {
            node.add_output_pin(
                &missing_tool_ref,
                &missing_tool_ref,
                "Tool Exec",
                VariableType::Execution,
            );
        }
    }
}
