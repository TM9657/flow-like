/// # Make OpenAI Function Node
/// Function call definitions or JSON Schemas are tedious to write by hand so this is an utility node to help you out.
/// Node execution can fail if the LLM produces an output that cannot be parsed as JSON schema.
/// If node execution succeeds, however, the output is *guaranteed* to be a valid OpenAI-like Function Call Definition with valid JSON schema in the "parameters" section.
use crate::ai::generative::llm::invoke_with_tools::extract_tagged;
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
use flow_like_model_provider::history::{
    History, HistoryFunction, HistoryMessage, Role, Tool, ToolType,
};
use flow_like_types::{anyhow, async_trait, json, Value};

#[derive(Default)]
pub struct SchemaFromExample {}

impl SchemaFromExample {
    pub fn new() -> Self {
        SchemaFromExample {}
    }
}

#[async_trait]
impl NodeLogic for SchemaFromExample {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "utils_json_make_schema",
            "Make Tool Schema",
            "Generate Tool Definitions for Tool Calls",
            "Utils/JSON",
        );
        node.add_icon("/flow/icons/repair.svg");

        node.add_input_pin("exec_in", "Input", "Trigger Pin", VariableType::Execution);

        node.add_input_pin(
            "example_json",
            "Example JSON",
            "Example JSON to infer schema from",
            VariableType::String,
        );

        node.add_output_pin(
            "exec_out",
            "Execution Output",
            "Execution Output",
            VariableType::Execution,
        );

        node.add_output_pin(
            "schema",
            "Schema",
            "Generated JSON Schema / Tool Definition",
            VariableType::Struct,
        );

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        // fetch inputs
        context.deactivate_exec_pin("exec_out").await?;
        let example_json: String = context.evaluate_pin("example_json").await?;

        let example_json: Value = json::from_str(&example_json)?;
        let schema = schemars::schema_for_value!(&example_json);
        let schema = json::to_string_pretty(&schema)?;
        let schema = json::from_str::<Value>(&schema)?;

        context.set_pin_value("schema", schema).await?;
        context.activate_exec_pin("exec_out").await?;
        Ok(())
    }
}
