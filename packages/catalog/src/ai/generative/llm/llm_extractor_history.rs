use async_trait::async_trait;
use flow_like::{
    bit::Bit,
    flow::{
        board::Board,
        execution::{LogLevel, context::ExecutionContext},
        node::{Node, NodeLogic},
        pin::PinOptions,
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_model_provider::history::History;
use flow_like_types::json::{self, Deserialize, Serialize};
use flow_like_types::{Value, anyhow};
use rig::completion::{Completion, ToolDefinition};
use rig::message::{AssistantContent, ToolCall, ToolChoice, ToolFunction};
use rig::tool::Tool;
use std::{fmt, sync::Arc};

#[derive(Default)]
pub struct LLMExtractHistoryNode {}

impl LLMExtractHistoryNode {
    pub fn new() -> Self {
        LLMExtractHistoryNode {}
    }
}

// --- Dynamic knowledge extraction submit tool that takes a runtime JSON Schema ---
#[derive(Debug, Deserialize, Serialize)]
struct DynamicSubmitTool {
    parameters: Value,
    output_schema: Value,
}

#[derive(Debug)]
struct SubmitError(String);

impl fmt::Display for SubmitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Schema validation failed: {}", self.0)
    }
}

impl std::error::Error for SubmitError {}

impl Tool for DynamicSubmitTool {
    const NAME: &'static str = "submit";
    type Error = SubmitError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Knowledge extraction submit tool. Return structured data that matches the provided schema.".to_string(),
            parameters: self.parameters.clone(),
        }
    }

    async fn call(&self, args: Self::Args) -> std::result::Result<Self::Output, Self::Error> {
        jsonschema::validate(&self.output_schema, &args)
            .map_err(|e| SubmitError(format!("{}", e)))?;
        Ok(args)
    }

    fn name(&self) -> String {
        Self::NAME.to_string()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ExtractionMode {
    Direct,
    Wrapped,
}

struct PreparedSchema {
    tool_parameters: Value,
    output_schema: Value,
    display: String,
    mode: ExtractionMode,
}

fn prepare_schema(raw: &str) -> flow_like_types::Result<PreparedSchema> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(anyhow!("Schema input cannot be empty"));
    }

    let user_json = json::from_str::<Value>(trimmed).map_err(|e| {
        anyhow!(
            "Schema must be valid JSON (either a JSON Schema or an example JSON). Parse error: {e}"
        )
    })?;

    let inferred = if jsonschema::meta::is_valid(&user_json) {
        user_json
    } else {
        let schema = schemars::schema_for_value!(&user_json);
        let string = json::to_string_pretty(&schema)?;
        json::from_str(&string)?
    };

    let display = json::to_string_pretty(&inferred).unwrap_or_else(|_| inferred.to_string());

    let mode = match inferred.get("type").and_then(|t| t.as_str()) {
        Some("object") => ExtractionMode::Direct,
        _ => ExtractionMode::Wrapped,
    };

    let tool_parameters = if mode == ExtractionMode::Direct {
        inferred.clone()
    } else {
        json::json!({
            "type": "object",
            "properties": {"value": inferred.clone()},
            "required": ["value"],
            "additionalProperties": false
        })
    };

    Ok(PreparedSchema {
        tool_parameters,
        output_schema: inferred,
        display,
        mode,
    })
}

#[async_trait]
impl NodeLogic for LLMExtractHistoryNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "llm_extractor_history",
            "AI Extractor from History",
            "Extracts structured data from LLM responses using a conversation history",
            "AI/Generative",
        );
        node.add_icon("/flow/icons/bot-invoke.svg");

        node.add_input_pin("exec_in", "Input", "Trigger Pin", VariableType::Execution);

        node.add_input_pin("model", "Model", "Model", VariableType::Struct)
            .set_schema::<Bit>()
            .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_input_pin(
            "schema",
            "Schema",
            "Tool Definition / JSON Schema for Structured Response, you can also paste an example JSON",
            VariableType::String,
        );

        node.add_input_pin(
            "history",
            "History",
            "Conversation history to be processed by the LLM",
            VariableType::Struct,
        )
        .set_schema::<History>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_output_pin(
            "exec_out",
            "Execution Output",
            "Execution Output",
            VariableType::Execution,
        );

        node.add_output_pin(
            "response",
            "Response",
            "Structured Response",
            VariableType::Struct,
        );

        node.set_long_running(true);

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;

        let model_bit = context.evaluate_pin::<Bit>("model").await?;
        let schema_str: String = context.evaluate_pin("schema").await?;
        let history: History = context.evaluate_pin("history").await?;

        let prepared_schema = prepare_schema(&schema_str)?;

        context.log_message(
            &format!("Using extraction mode: {:?}", prepared_schema.mode),
            LogLevel::Debug,
        );

        let preamble = "You are a knowledge extraction assistant. Extract data by calling the 'submit' tool with structured data matching the provided schema.";

        let (prompt, chat_history) = history
            .extract_prompt_and_history()
            .map_err(|e| anyhow!("Failed to convert history into rig messages: {e}"))?;

        let agent_builder = model_bit
            .agent(context, &Some(history))
            .await?
            .preamble(preamble)
            .tool(DynamicSubmitTool {
                parameters: prepared_schema.tool_parameters,
                output_schema: prepared_schema.output_schema.clone(),
            })
            .tool_choice(ToolChoice::Required);

        let agent = agent_builder.build();

        let response = agent
            .completion(prompt, chat_history.into_iter().collect())
            .await
            .map_err(|e| anyhow!("Model completion failed: {}", e))?
            .send()
            .await
            .map_err(|e| anyhow!("Failed to send completion request: {}", e))?;

        let mut last_args: Option<Value> = None;
        for content in response.choice {
            if let AssistantContent::ToolCall(ToolCall {
                function: ToolFunction {
                    name, arguments, ..
                },
                ..
            }) = content
                && name == "submit"
            {
                last_args = Some(arguments);
            }
        }

        let args = last_args.ok_or_else(|| {
            anyhow!("Model did not return a 'submit' tool call. Ensure the model supports function calling.")
        })?;

        let extracted = match prepared_schema.mode {
            ExtractionMode::Direct => args,
            ExtractionMode::Wrapped => args
                .get("value")
                .cloned()
                .ok_or_else(|| anyhow!("Tool call missing 'value' field in wrapped mode"))?,
        };

        context.log_message("Successfully extracted structured data", LogLevel::Debug);

        context.set_pin_value("response", extracted).await?;
        context.activate_exec_pin("exec_out").await?;
        Ok(())
    }

    async fn on_update(&self, node: &mut Node, _board: Arc<Board>) {
        node.error = None;

        let schema_value = node
            .get_pin_by_name("schema")
            .and_then(|pin| {
                pin.default_value
                    .as_ref()
                    .and_then(|bytes| json::from_slice::<Value>(bytes).ok())
            })
            .and_then(|value| value.as_str().map(|s| s.to_string()));

        match schema_value {
            Some(raw) if raw.trim().is_empty() => {
                node.error = Some("Schema input cannot be empty".to_string());
            }
            Some(raw) => {
                match prepare_schema(&raw) {
                    Ok(prepared) => {
                        // Check if the original input was a valid meta schema
                        let user_json = json::from_str::<Value>(&raw);
                        if let Ok(user_json) = user_json
                            && !jsonschema::meta::is_valid(&user_json)
                        {
                            // Input was not a valid schema, update with inferred schema
                            if let Some(pin) = node.get_pin_mut_by_name("schema") {
                                let schema_str = json::to_string_pretty(&prepared.output_schema)
                                    .unwrap_or_else(|_| prepared.output_schema.to_string());
                                let _ = pin.set_default_value(Some(json::json!(schema_str)));
                            }
                        }
                    }
                    Err(err) => {
                        node.error = Some(format!("Schema error: {}", err));
                    }
                }
            }
            None => {
                node.error = Some("Schema input cannot be empty".to_string());
            }
        }
    }
}
