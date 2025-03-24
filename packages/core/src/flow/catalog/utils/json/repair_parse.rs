use crate::{
    flow::{
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        variable::VariableType,
    },
    state::FlowLikeState,
    utils::json::parse_malformed_json,
};
use async_trait::async_trait;

#[derive(Default)]
pub struct RepairParseNode {}

impl RepairParseNode {
    pub fn new() -> Self {
        RepairParseNode {}
    }
}

#[async_trait]
impl NodeLogic for RepairParseNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "repair_parse",
            "Repair Parse JSON",
            "Attempts to repair and parse potentially malformed JSON",
            "Utils/JSON",
        );

        node.add_icon("/flow/icons/repair.svg");

        node.add_input_pin(
            "exec_in",
            "Input",
            "Initiate Execution",
            VariableType::Execution,
        );

        node.add_input_pin(
            "json_string",
            "JSON String",
            "String containing potentially malformed JSON",
            VariableType::String,
        );

        node.add_output_pin(
            "exec_out",
            "Output",
            "Execution continues if parsing succeeds",
            VariableType::Execution,
        );

        node.add_output_pin(
            "result",
            "Result",
            "The parsed JSON structure",
            VariableType::Struct,
        );

        node.add_output_pin(
            "failed",
            "Failed",
            "Execution continues if parsing fails",
            VariableType::Execution,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;
        context.activate_exec_pin("failed").await?;

        let json_string: String = context.evaluate_pin("json_string").await?;

        match parse_malformed_json(&json_string) {
            Ok(value) => {
                context.set_pin_value("result", value).await?;
                context.activate_exec_pin("exec_out").await?;
                context.deactivate_exec_pin("failed").await?;
            }
            Err(err) => {
                context.log_message(
                    &format!("Failed to parse JSON: {}", err),
                    crate::flow::execution::LogLevel::Error,
                );
            }
        }

        Ok(())
    }
}
