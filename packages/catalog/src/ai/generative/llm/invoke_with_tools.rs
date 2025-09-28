/// # Invoke LLMs With Tools
/// Make an LLM Invoke / Chat Completion Request allowing for tool calls with dedicated output pins.
/// Iterates over all tool calls in LLM response.
/// Once no more tool calls, stop execution.
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
use flow_like_model_provider::{
    history::{History, Tool, ToolCall, ToolChoice},
    response::Response,
};
use flow_like_types::{Error, Value, anyhow, async_trait, json, regex::Regex};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

const SP_TEMPLATE_AUTO: &str = r#"
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
"#;

const SP_TEMPLATE_REQUIRED: &str = r#"
# Instruction
You are a helpful assistant with access to the tools below.

# Tools
You *MUST* use one of the tools below to make your response.

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

You *MUST* wrap your tool use json data in these xml tags: <tooluse></tooluse>.

Do *NOT* use code blocks.

Wrap every tool use in a pair of xml tags <tooluse></tooluse>.

You *MUST* use at least one tool.
"#;

const SP_TEMPLATE_SPECIFIC: &str = r#"
# Instruction
You are a helpful assistant with access to the tool below.

# Tool
You *MUST* use the tools below to make your response.

## Schema
TOOL_STR

## Tool Use Format
<tooluse>
    {
        "name": "<name of the tool>",
        "arguments": "<key: value dict for args as defined by schema of the tool>"
    }
</tooluse>

# Response Format
Your tool use json data within the <tooluse></tooluse> will be validated by the tool json schemas above.

The tool use data string inside the <tooluse></tooluse> tags *MUST* be compliant with the tool json schemas above.

You *MUST* wrap your tool use json data in these xml tags: <tooluse></tooluse>.

Do *NOT* use code blocks.

Wrap your tool use in a pair of xml tags <tooluse></tooluse>.

You *MUST* use the tool specified above to make your response.
"#;

/// Extract tagged substrings, e.g. Hello, <tool>extract this</tool> and <tool>this</tool>, good bye.
pub fn extract_tagged_simple(text: &str, tag: &str) -> Result<Vec<String>, Error> {
    let pattern = format!(r"(?s)<{tag}>(.*?)</{tag}>", tag = regex::escape(tag));
    let re = Regex::new(&pattern)?;
    Ok(re
        .captures_iter(text)
        .filter_map(|caps| caps.get(1).map(|m| m.as_str().to_string()))
        .collect())
}

/// Extract tagged substrings, e.g. Hello, <tool>extract this</tool> and <tool>this</tool>, good bye.
/// This is a more robust version that ignores tags not being closed.
pub fn extract_tagged(text: &str, tag: &str) -> Result<Vec<String>, Error> {
    let open_tag = format!("<{tag}>");
    let close_tag = format!("</{tag}>");

    // 1) Find all open-tag positions
    let mut starts = Vec::new();
    let mut pos = 0;
    while let Some(idx) = text[pos..].find(&open_tag) {
        let real = pos + idx;
        starts.push(real);
        pos = real + open_tag.len();
    }

    // 2) Find all close-tag positions
    let mut ends = Vec::new();
    let mut pos = 0;
    while let Some(idx) = text[pos..].find(&close_tag) {
        let real = pos + idx;
        ends.push(real);
        pos = real + close_tag.len();
    }

    // 3) For each opener, match to the first unused closer that comes after it,
    //    but only if there’s no *other* opener in between them.
    let mut used_ends = vec![false; ends.len()];
    let mut out = Vec::new();

    for &start in &starts {
        let content_start = start + open_tag.len();
        // find the first unused closing tag after this opener
        if let Some((ei, &end_pos)) = ends
            .iter()
            .enumerate()
            .find(|&(i, &e)| !used_ends[i] && e > content_start)
        {
            // check for any *other* opener nested between this opener and that closer:
            let has_inner_opener = starts.iter().any(|&other| other > start && other < end_pos);

            if has_inner_opener {
                // this opener is “orphaned” by an inner start—skip it
                continue;
            }

            // otherwise, we have a proper pair: extract, mark this closer used
            let slice = &text[content_start..end_pos];
            out.push(slice.to_string());
            used_ends[ei] = true;
        }
    }
    Ok(out)
}

#[derive(Default)]
pub struct InvokeLLMWithToolsNode {}

impl InvokeLLMWithToolsNode {
    pub fn new() -> Self {
        InvokeLLMWithToolsNode {}
    }
}

#[async_trait]
impl NodeLogic for InvokeLLMWithToolsNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "invoke_llm_with_tools",
            "Invoke with Tools",
            "Invoke LLM with Tool Cals",
            "AI/Generative",
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

        node.add_input_pin(
            "tool_choice",
            "Tool Choice",
            "Tool Choice Mode",
            VariableType::String,
        )
        .set_options(
            PinOptions::new()
                .set_valid_values(vec![
                    "Auto".to_string(),
                    "Required".to_string(),
                    "None".to_string(),
                ])
                .build(),
        )
        .set_default_value(Some(json::json!("Auto")));

        node.add_output_pin("exec_done", "Done", "Done Pin", VariableType::Execution);

        node.add_output_pin(
            "response",
            "Response",
            "Final response if not tool call made",
            VariableType::Struct,
        )
        .set_schema::<Response>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_output_pin(
            "tool_call_args",
            "Tool Call Args",
            "Tool Call Arguments",
            VariableType::Struct,
        );

        node.set_long_running(true);

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_done").await?;

        // fetch inputs
        let model_bit = context.evaluate_pin::<Bit>("model").await?;
        let tools_str: String = context.evaluate_pin("tools").await?;
        let tool_choice: String = context.evaluate_pin("tool_choice").await?;
        let tool_choice = match tool_choice.as_str() {
            "Required" => ToolChoice::Required,
            "None" => ToolChoice::None,
            _ => ToolChoice::Auto,
        };

        // validate tools + deactivate all function exec output pins
        let tools: Vec<Tool> = match json::from_str(&tools_str) {
            Ok(value) => value,
            Err(e) => return Err(anyhow!("Failed to parse tools: {e:?}")),
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
            match tool_choice {
                ToolChoice::Auto => SP_TEMPLATE_AUTO.replace("TOOLS_STR", &tools_str),
                ToolChoice::Required => SP_TEMPLATE_REQUIRED.replace("TOOLS_STR", &tools_str),
                _ => String::from(""),
            }
        } else {
            String::from("")
        };
        let mut history = context.evaluate_pin::<History>("history").await?;
        let system_prompt = match history.get_system_prompt() {
            Some(system_prompt) => {
                format!("{}\n\n{}", system_prompt, system_prompt_tools) // handle previously set system prompts
            }
            None => system_prompt_tools,
        };
        history.set_system_prompt(system_prompt.to_string());
        context.log_message(
            &format!("system prompt: {}", system_prompt),
            LogLevel::Debug,
        );

        // generate response
        let response = {
            // load model
            let model_factory = context.app_state.lock().await.model_factory.clone();
            let model = model_factory
                .lock()
                .await
                .build(&model_bit, context.app_state.clone(), context.token.clone())
                .await?;
            model.invoke(&history, None).await?
        }; // drop model

        // parse response
        let mut response_string = "".to_string();
        if let Some(response) = response.last_message() {
            response_string = response.content.clone().unwrap_or("".to_string());
        }
        context.log_message(
            &format!("llm response: '{}'", &response_string),
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
            if let ToolChoice::None = tool_choice {
                return Err(anyhow!("LLM made tool calls but tool choice is None!"));
            };

            //let tool_call_id_pin = context.get_pin_by_name("tool_call_id").await?;
            let tool_call_args_pin = context.get_pin_by_name("tool_call_args").await?;
            for tool_call in tool_calls.iter() {
                let tool_call_args: Value = json::from_str(&tool_call.function.arguments)?;
                context.log_message(
                    &format!("exec tool {}", &tool_call.function.name),
                    LogLevel::Debug,
                );

                // deactivate all tool exec pins
                for tool in &tools {
                    context.deactivate_exec_pin(&tool.function.name).await?
                }

                // set tool args + activate tool exec pin
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

        // LLM doesn't want to make any tool calls -> return response as-is
        } else {
            match tool_choice {
                ToolChoice::Required => {
                    return Err(anyhow!(
                        "LLM made no tool calls but at least one is required!"
                    ));
                }
                _ => {
                    context
                        .set_pin_value("response", json::json!(response))
                        .await?;
                }
            }
        }

        // all done
        context.activate_exec_pin("exec_done").await?;
        Ok(())
    }

    async fn on_update(&self, node: &mut Node, _board: Arc<Board>) {
        node.error = None;
        let current_tool_exec_pins: Vec<_> = node
            .pins
            .values()
            .filter(|p| {
                p.pin_type == PinType::Output
                    && (p.name != "exec_done" && p.name != "response" && p.name != "tool_call_args") // p.description == "Tool Exec" doesn't seem to work as filter cond
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
