/// # Simple Agent Node
/// This is an LLM-controlled while loop over an arbitrary number of flow-leafes with back-propagation of leaf outputs into the agent.
/// Recursive LLM-invokes until no more tool calls are made or recursion limit hit.
/// Effectively, this node allows the LLM to control it's own execution until further human input required.
use crate::ai::generative::llm::invoke_with_tools::extract_tagged;
use crate::utils::json::parse_with_schema::tool_call_from_str;
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
use flow_like_model_provider::history::ToolCall;
use flow_like_model_provider::{
    history::{Content, ContentType, History, HistoryMessage, MessageContent, Role, Tool},
    response::Response,
};

use flow_like_types::{Error, Value, anyhow, async_trait, json};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

const SYSTEM_PROMPT_TEMPLATE: &str = r#"
# Instruction
You are a helpful assistant with access to the tools below.

# Tools
Here are the schemas for the tools you *can* use:

## Schemas
TOOLS_STR

## Tool Use Format
<tooluse>
    {
        "name": "<name of the tool you want to use>",
        "arguments": "<key: value dict for args as defined by schema of the tool you want to use>"
    }
</tooluse>

# Response Format
Your tool use json data within the <tooluse></tooluse> will be validated by the tool json schemas above.

The tool use data string inside the <tooluse></tooluse> tags *MUST* be compliant with the tool json schemas above.

If you want to use a tool you *MUST* wrap your tool use json data in these xml tags: <tooluse></tooluse>.

Do *NOT* use code blocks.

Wrap every tool use in a pair of xml tags <tooluse></tooluse>.

Once all tool outputs have been gathered, reply back to the original user input.
"#;

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

        // future: at some point we could allow for parallel tool execution
        // for now, we only implement sequential processing in a loop to avoid writing to global variables at the same time
        //node.add_input_pin("thread_model", "Threads", "Threads", VariableType::String)
        //    .set_default_value(Some(json::json!("tasks")))
        //    .set_options(
        //        PinOptions::new()
        //            .set_valid_values(vec!["sequential".to_string()])
        //            .build(),
        //    );

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

        // fetch inputs
        let recursion_limit: u64 = context.evaluate_pin("max_iter").await?;
        let model_bit = context.evaluate_pin::<Bit>("model").await?;
        let tools_str: String = context.evaluate_pin("tools").await?;

        // validate tools + deactivate all function exec output pins
        let tools: Vec<Tool> = match json::from_str(&tools_str) {
            Ok(tools) => tools,
            Err(err) => return Err(anyhow!("Failed to parse tools: {err:?}")),
        };
        for tool in &tools {
            context.deactivate_exec_pin(&tool.function.name).await?
        }

        // log model name
        if let Some(meta) = model_bit.meta.get("en") {
            context.log_message(&format!("Loading model {:?}", meta.name), LogLevel::Debug);
        }

        // render system prompt with add-on for tool definitions
        let system_prompt_tools = if !tools.is_empty() {
            SYSTEM_PROMPT_TEMPLATE.replace("TOOLS_STR", &tools_str) // todo: serlialize tools instead?
        } else {
            String::from("")
        };
        let history = context.evaluate_pin::<History>("history").await?;
        let system_prompt = match history.get_system_prompt() {
            Some(system_prompt) => {
                format!("{}\n\n{}", system_prompt, system_prompt_tools) // handle previously set system prompts
            }
            None => system_prompt_tools,
        };
        context.log_message(
            &format!("system prompt: {}", system_prompt),
            LogLevel::Debug,
        );

        // Loop until no more tool cals or max recursion limit hit
        let mut previous_external_history = History::new(history.model.clone(), vec![]);
        let mut internal_history = History::new(history.model.clone(), vec![]);
        let mut unanswered_tool_calls: HashMap<String, String> = HashMap::new();
        for agent_iteration in 0..recursion_limit {
            context.log_message(
                &format!("[agent iter {}] agent iteration", agent_iteration),
                LogLevel::Debug,
            );

            // re-evaluate history + set system prompt
            let mut external_history = context.evaluate_pin::<History>("history").await?;
            external_history.set_system_prompt(system_prompt.to_string());
            context.log_message(
                &format!(
                    "[agent iter {}] previous external history: {}",
                    agent_iteration, &previous_external_history
                ),
                LogLevel::Debug,
            );
            context.log_message(
                &format!(
                    "[agent iter {}] previous internal history: {}",
                    agent_iteration, &internal_history
                ),
                LogLevel::Debug,
            );

            // append new messages to internal history
            // validate whether an incoming tool output message can be associated to a tool call of a previous assistant message
            // failing to do so can lead to LLMs not seeing results or even bad requests for cloud-model provider
            let offset = previous_external_history.messages.len();
            for new_message in external_history.messages.iter().skip(offset) {
                match new_message.role {
                    Role::Tool => {
                        let content_str = if let Some(tool_call_id) = &new_message.tool_call_id {
                            if let Some(tool_name) = unanswered_tool_calls.remove(tool_call_id) {
                                format!("[tooloutput] [{}]: {}", tool_name, new_message.as_str())
                            } else {
                                context.log_message(&format!("Couldn't link new tool message with id {} to any previous tool call", tool_call_id), LogLevel::Warn);
                                format!("[tooloutput]: {}", new_message.as_str())
                            }
                        } else {
                            context.log_message(
                                "New tool message is missing a tool call id",
                                LogLevel::Warn,
                            );
                            format!("[tooloutput]: {}", new_message.as_str())
                        };
                        let message = HistoryMessage {
                            role: Role::User,
                            content: MessageContent::Contents(vec![Content::Text {
                                content_type: ContentType::Text,
                                text: content_str,
                            }]),
                            tool_call_id: None,
                            tool_calls: None,
                            name: None,
                            annotations: None,
                        };
                        internal_history.messages.push(message);
                    }
                    _ => {
                        // if there are tool calls from a previous iteration not answered by tool outputs we warn the user
                        if !unanswered_tool_calls.is_empty() {
                            context.log_message(&format!("There are open tool calls but incoming message hasn't role 'tool' but {:?} - this can lead to non-optimal performance.", new_message.role), LogLevel::Warn);
                        }
                        // if there aren't any tool calls (yet) to answer it's fine
                        internal_history.messages.push(new_message.clone());
                    }
                }
            }
            context.log_message(
                &format!(
                    "[agent iter {}] updated  external history: {}",
                    agent_iteration, &external_history
                ),
                LogLevel::Debug,
            );
            context.log_message(
                &format!(
                    "[agent iter {}] updated  internal history: {}",
                    agent_iteration, &internal_history
                ),
                LogLevel::Debug,
            );
            previous_external_history = external_history;

            // generate response
            let response = {
                // load model
                let model_factory = context.app_state.lock().await.model_factory.clone();
                let model = model_factory
                    .lock()
                    .await
                    .build(&model_bit, context.app_state.clone())
                    .await?;
                model.invoke(&internal_history, None).await?
            }; // drop model

            // parse response
            let mut response_string = "".to_string();
            if let Some(response) = response.last_message() {
                response_string = response.content.clone().unwrap_or("".to_string());
            }
            context.log_message(
                &format!(
                    "[agent iter {}] llm response: '{}'",
                    agent_iteration, &response_string
                ),
                LogLevel::Debug,
            );

            // parse tool calls (if any)
            let tool_calls = if response_string.contains("<tooluse>") {
                let tool_calls_str = extract_tagged(&response_string, "tooluse")?;
                let tool_calls: Result<Vec<ToolCall>, Error> = tool_calls_str
                    .iter()
                    .map(|tool_call_str| tool_call_from_str(&tools, tool_call_str))
                    .collect();
                tool_calls?
            } else {
                vec![]
            };

            // LLM wants to make tool calls -> execute subcontexts
            if !tool_calls.is_empty() {
                let tool_call_id_pin = context.get_pin_by_name("tool_call_id").await?;
                let tool_call_args_pin = context.get_pin_by_name("tool_call_args").await?;
                for tool_call in tool_calls.iter() {
                    let tool_call_args: Value = json::from_str(&tool_call.function.arguments)?;
                    context.log_message(
                        &format!(
                            "[agent iter {}] exec tool {}",
                            agent_iteration, &tool_call.function.name
                        ),
                        LogLevel::Debug,
                    );

                    // deactivate all tool exec pins
                    for tool in &tools {
                        context.deactivate_exec_pin(&tool.function.name).await?
                    }

                    // set tool args + activate tool exec pin
                    tool_call_id_pin
                        .lock()
                        .await
                        .set_value(json::json!(&tool_call.id))
                        .await;
                    tool_call_args_pin
                        .lock()
                        .await
                        .set_value(tool_call_args)
                        .await;
                    context.activate_exec_pin(&tool_call.function.name).await?;

                    // execute tool subcontext
                    let tool_exec_pin = context.get_pin_by_name(&tool_call.function.name).await?;
                    let tool_flow = tool_exec_pin.lock().await.get_connected_nodes().await;
                    for node in &tool_flow {
                        let mut sub_context = context.create_sub_context(node).await;
                        let run = InternalNode::trigger(&mut sub_context, &mut None, true).await;

                        sub_context.end_trace();
                        context.push_sub_context(&mut sub_context);
                        if run.is_err() {
                            let error = run.err().unwrap();
                            context.log_message(
                                &format!(
                                    "Error executing tool {}: {:?}",
                                    &tool_call.function.name, error
                                ),
                                LogLevel::Error,
                            );
                        }
                    }
                }
                // deactivate all tool exec pins
                for tool in &tools {
                    context.deactivate_exec_pin(&tool.function.name).await?
                }

            // LLM doesn't want to make any tool calls -> return final response
            } else {
                context
                    .set_pin_value("response", json::json!(response))
                    .await?; // todo: remove prefix from response struct
                context.activate_exec_pin("exec_done").await?;
                return Ok(());
            }

            // prep for next iteration
            // -> track open tool calls
            for tool_call in tool_calls.iter() {
                unanswered_tool_calls.insert(tool_call.id.clone(), tool_call.function.name.clone());
            }

            // -> append own response as assistant message to internal history
            let ai_message = HistoryMessage {
                role: Role::Assistant,
                content: MessageContent::Contents(vec![Content::Text {
                    content_type: ContentType::Text,
                    text: response_string,
                }]),
                name: None,
                tool_call_id: None,
                tool_calls: Some(tool_calls),
                annotations: None,
            };
            internal_history.messages.push(ai_message);
        }
        return Err(anyhow!("Max recursion limit hit"));
    }

    async fn on_update(&self, node: &mut Node, board: Arc<Board>) {
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
