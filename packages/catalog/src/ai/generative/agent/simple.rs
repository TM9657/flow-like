/// # Simple Agent Node
/// Recursive LLM-invokes until no more tool calls are made or recursion limit hit.
/// Effectively, this is an LLM-controlled while loop over an arbitrary number of flow-leafes with back-propagation of leaf outputs into the agent.
use crate::ai::generative::llm::invoke_with_tools::extract_tagged;
use crate::utils::json::parse_with_schema::{
    OpenAIFunction, OpenAIToolCall, validate_openai_functions_str, validate_openai_tool_call_str,
};
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
    history::{Content, ContentType, History, HistoryMessage, MessageContent, Role},
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
        "args": "<key: value dict for args as defined by schema of the tool you want to use>"
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
        node.add_icon("/flow/icons/bot-invoke.svg");

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
        let functions = validate_openai_functions_str(&tools_str)?;
        for function in &functions {
            context.deactivate_exec_pin(&function.name).await?
        }

        // log model name
        if let Some(meta) = model_bit.meta.get("en") {
            context.log_message(&format!("Loading model {:?}", meta.name), LogLevel::Debug);
        }

        // render system prompt with add-on for tool definitions
        let system_prompt_tools = if functions.len() > 0 {
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
        for agent_iteration in 0..recursion_limit {
            context.log_message(
                &format!("[agent iter {}] agent iteration", agent_iteration),
                LogLevel::Debug,
            );

            // re-evaluate history + set system prompt
            let mut external_history = context.evaluate_pin::<History>("history").await?;
            external_history.set_system_prompt(system_prompt.to_string());

            // append new messages to internal history
            // todo: validate tool call followed by tool output
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
            let n1 = previous_external_history.messages.len();
            let n2 = external_history.messages.len();
            for i in n1..n2 {
                let new_message = external_history.messages[i].clone();
                internal_history.messages.push(new_message);
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

            // parse tool cals (if any)
            let tool_calls = if response_string.contains("<tooluse>") {
                let tool_calls_str = extract_tagged(&response_string, "tooluse")?;
                let tool_calls: Result<Vec<OpenAIToolCall>, Error> = tool_calls_str
                    .iter()
                    .map(|tool_call_str| validate_openai_tool_call_str(&functions, tool_call_str))
                    .collect();
                tool_calls?
            } else {
                vec![]
            };

            // LLM wants to make tool calls -> execute subcontexts
            if tool_calls.len() > 0 {
                let tool_call_id_pin = context.get_pin_by_name("tool_call_id").await?;
                let tool_call_args_pin = context.get_pin_by_name("tool_call_args").await?;
                for tool_call in tool_calls.iter() {
                    context.log_message(
                        &format!(
                            "[agent iter {}] exec tool {}",
                            agent_iteration, &tool_call.name
                        ),
                        LogLevel::Debug,
                    );

                    // deactivate all tool exec pins
                    for function in &functions {
                        context.deactivate_exec_pin(&function.name).await?
                    }

                    // set tool args + activate tool exec pin
                    tool_call_id_pin
                        .lock()
                        .await
                        .set_value(json::json!("abc"))
                        .await;
                    tool_call_args_pin
                        .lock()
                        .await
                        .set_value(json::json!(tool_call.args))
                        .await;
                    context.activate_exec_pin(&tool_call.name).await?;

                    // execute tool subcontext
                    let tool_exec_pin = context.get_pin_by_name(&tool_call.name).await?;
                    let tool_flow = tool_exec_pin.lock().await.get_connected_nodes().await;
                    for node in &tool_flow {
                        let mut sub_context = context.create_sub_context(node).await;
                        let run = InternalNode::trigger(&mut sub_context, &mut None, true).await;

                        sub_context.end_trace();
                        context.push_sub_context(sub_context);
                        if run.is_err() {
                            let error = run.err().unwrap();
                            context.log_message(
                                &format!("Error executing tool {}: {:?}", &tool_call.name, error),
                                LogLevel::Error,
                            );
                        }
                    }
                }
                // deactivate all tool exec pins
                for function in &functions {
                    context.deactivate_exec_pin(&function.name).await?
                }

            // LLM doesn't want to make any tool calls -> return final response
            } else {
                context
                    .set_pin_value("response", json::json!(response))
                    .await?; // todo: remove prefix from response struct
                context.activate_exec_pin("exec_done").await?;
                return Ok(());
            }

            // append response as assistant message to internal history
            let ai_message = HistoryMessage {
                role: Role::Assistant,
                content: MessageContent::Contents(vec![Content::Text {
                    content_type: ContentType::Text,
                    text: response_string,
                }]),
                name: None,
                tool_call_id: None,
                tool_calls: None, // todo: use tool calls + tool messages
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
                    && (p.name != "exec_done" && p.name != "response" && p.name != "tool_call_id" && p.name != "tool_call_args") // p.description == "Tool Exec" doesn't seem to work as filter cond
            })
            .collect();

        let schema_str: String = node
            .get_pin_by_name("tools")
            .and_then(|pin| pin.default_value.clone())
            .and_then(|bytes| flow_like_types::json::from_slice::<Value>(&bytes).ok())
            .and_then(|json| json.as_str().map(ToOwned::to_owned))
            .unwrap_or_default();

        let mut current_tool_exec_refs = current_tool_exec_pins
            .iter()
            .map(|p| (p.name.clone(), *p))
            .collect::<HashMap<_, _>>();

        let update_tools: Vec<OpenAIFunction> = match validate_openai_functions_str(&schema_str) {
            Ok(tools) => tools,
            Err(err) => {
                node.error = Some(format!("Failed to parse tools: {err:?}").to_string());
                return;
            }
        };

        let mut all_tool_exec_refs = HashSet::new();
        let mut missing_tool_exec_refs = HashSet::new();

        for update_tool in update_tools {
            all_tool_exec_refs.insert(update_tool.name.clone());
            if current_tool_exec_refs.remove(&update_tool.name).is_none() {
                missing_tool_exec_refs.insert(update_tool.name.clone());
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
