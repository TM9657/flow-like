use crate::{
    flow::{
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        variable::VariableType,
    },
    state::FlowLikeState,
};
use async_trait::async_trait;
use serde_json::json;

#[derive(Default)]
pub struct ClampIntegerNode {}

impl ClampIntegerNode {
    pub fn new() -> Self {
        ClampIntegerNode {}
    }
}

#[async_trait]
impl NodeLogic for ClampIntegerNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "int_clamp",
            "Clamp",
            "Clamps an integer within a range",
            "Math/Int",
        );
        node.add_icon("/flow/icons/sigma.svg");

        node.add_input_pin("integer", "Integer", "Input Integer", VariableType::Integer);
        node.add_input_pin("min", "Min", "Minimum Value", VariableType::Integer);
        node.add_input_pin("max", "Max", "Maximum Value", VariableType::Integer);

        node.add_output_pin("clamped", "Clamped", "Clamped Value", VariableType::Integer);

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        let integer: i64 = context.evaluate_pin("integer").await?;
        let min: i64 = context.evaluate_pin("min").await?;
        let max: i64 = context.evaluate_pin("max").await?;

        let clamped = integer.clamp(min, max);

        context.set_pin_value("clamped", json!(clamped)).await?;
        Ok(())
    }
}
