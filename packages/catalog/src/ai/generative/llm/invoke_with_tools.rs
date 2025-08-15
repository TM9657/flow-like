
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
use flow_like_model_provider::{history::{Content, ContentType, History, HistoryMessage, MessageContent, Role}, response::Response};

use flow_like_types::{anyhow, async_trait, json, regex::Regex, Error, Value};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

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

        // future: at some point we could allow for parallel tool execution
        // for now, we only implement sequential processing in a loop to avoid writing to global variables at the same time
        node.add_input_pin("thread_model", "Threads", "Threads", VariableType::String)
            .set_default_value(Some(json::json!("tasks")))
            .set_options(
                PinOptions::new()
                    .set_valid_values(vec!["sequential".to_string()])
                    .build(),
            );

        node.add_input_pin(
            "max_iter",
            "Maxium Iterations",
            "Maximum Number of Iterations (Recursion Limit)",
            VariableType::Integer,
        )
        .set_default_value(Some(json::json!(15)));

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
            "tool_args",
            "Tool Args",
            "Tool Call Arguments",
            VariableType::Struct,
        );

        node.set_long_running(true);

        return node;
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

        Ok(())
    }

    async fn on_update(&self, node: &mut Node, board: Arc<Board>) {
        let current_tool_exec_pins: Vec<_> = node
            .pins
            .values()
            .filter(|p| {
                p.pin_type == PinType::Output
                    && (p.name != "exec_done" && p.name != "response" && p.name != "tool_args") // p.description == "Tool Exec" doesn't seem to work as filter cond
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
