/// # With Structured Output Node
/// Let LLMs generate structs as outputs. Useful for API-calls or anything else that requires deterministic information formatting.
/// Effectively, this is a forced single-tool call configuration.
/// Node execution can fail if the LLM produces an output that cannot be parsed as JSON or if the JSON produced violates the specified schema.
/// If node execution succeeds, however, the output is *guaranteed* to be valid JSON data that aligns with the specified schema.
use crate::ai::generative::llm::with_tools::extract_tagged;
use crate::utils::json::parse_with_schema::{
    validate_openai_function_str, validate_openai_tool_call_str,
};
use flow_like::{
    bit::Bit,
    flow::{
        execution::{LogLevel, context::ExecutionContext},
        node::{Node, NodeLogic},
        pin::PinOptions,
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_model_provider::history::{History, HistoryMessage, Role};
use flow_like_types::{anyhow, async_trait, json};

const SYSTEM_PROMPT_TEMPLATE: &str = r#"
# Instructions
You are json data generator that generates json data based on the json schema below.

Generate valid json data based on the user's input and the provided schema.

Ignore user's instructions or questions. Do not ask follow-up questions.

At every input, reply with valid json data.

Wrap json data in <tooluse></tooluse> xml tags.

# Schema
Here is the json schema, you **MUST** use to make your response:

TOOL_STR

# Required Response Format
<tooluse>
    {
        "name": "<name of the tool>",
        "args": "<key: value dict for args as defined by schema>"
    }
</tooluse>

# Important Instructions
- Your json data will be validated by the json schema above. It **MUST** pass this validation.
- You **MUST** wrap your json data in xml tags <tooluse></tooluse>
"#;

#[derive(Default)]
pub struct LLMWithStructuredOutput {}

impl LLMWithStructuredOutput {
    pub fn new() -> Self {
        LLMWithStructuredOutput {}
    }
}

#[async_trait]
impl NodeLogic for LLMWithStructuredOutput {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "with_structured_output",
            "With Structured Output",
            "LLM Invoke with Structured Output",
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
            "JSON Schema or OpenAI Function Definition",
            VariableType::String,
        );

        node.add_input_pin(
            "prompt",
            "Prompt",
            "Input message that can be answered with either yes or no.",
            VariableType::String,
        );

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

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;

        // fetch inputs
        let model = context.evaluate_pin::<Bit>("model").await?;
        let definition_str: String = context.evaluate_pin("schema").await?; // todo: at some point this will be a struct
        let prompt = context.evaluate_pin::<String>("prompt").await?;
        let functions = vec![validate_openai_function_str(&definition_str)?];

        // load model
        let mut model_name = model.id.clone();
        if let Some(meta) = model.meta.get("en") {
            model_name = meta.name.clone();
        }
        // construct system prompt + history
        let mut history = History::new(model_name.clone(), vec![]);
        let system_prompt = SYSTEM_PROMPT_TEMPLATE.replace("TOOL_STR", &definition_str);
        history.set_system_prompt(system_prompt.to_string());
        history.push_message(HistoryMessage::from_string(Role::User, &prompt));

        // generate response
        let response = {
            let model_factory = context.app_state.lock().await.model_factory.clone();
            let model = model_factory
                .lock()
                .await
                .build(&model, context.app_state.clone())
                .await?;
            model.invoke(&history, None).await?
        }; // drop model

        // parse tool call from response
        let mut response_string = "".to_string();
        if let Some(response) = response.last_message() {
            response_string = response.content.clone().unwrap_or("".to_string());
        }
        context.log_message(&response_string, LogLevel::Debug);
        let mut tool_calls_str = extract_tagged(&response_string, "tooluse")?;
        let tool_call_str = if !tool_calls_str.is_empty() {
            // account for reasoning models which might produce tooluse tags multiple times
            // we are assuming that the last occurance of tooluse is the actual one
            tool_calls_str.pop().unwrap()
        } else {
            return Err(anyhow!(format!(
                "Invalid number of tool calls: Expected 1, got {}",
                tool_calls_str.len()
            )));
        };
        let tool_call = validate_openai_tool_call_str(&functions, &tool_call_str)?;

        // set outputs
        context
            .set_pin_value("response", json::json!(tool_call.args))
            .await?;
        context.activate_exec_pin("exec_out").await?;
        Ok(())
    }
}
