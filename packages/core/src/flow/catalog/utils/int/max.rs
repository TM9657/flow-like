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
pub struct MaxIntegerNode {}

impl MaxIntegerNode {
    pub fn new() -> Self {
        MaxIntegerNode {}
    }
}

#[async_trait]
impl NodeLogic for MaxIntegerNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "int_max",
            "Max",
            "Returns the larger of two integers",
            "Math/Int",
        );
        node.add_icon("/flow/icons/sigma.svg");

        node.add_input_pin(
            "integer1",
            "Integer 1",
            "Input Integer",
            VariableType::Integer,
        );
        node.add_input_pin(
            "integer2",
            "Integer 2",
            "Input Integer",
            VariableType::Integer,
        );

        node.add_output_pin(
            "maximum",
            "Maximum",
            "The larger of the two integers",
            VariableType::Integer,
        );

        return node;
    }

    async fn run(&mut self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        let integer1: i64 = context.evaluate_pin("integer1").await?;
        let integer2: i64 = context.evaluate_pin("integer2").await?;

        let maximum = integer1.max(integer2);

        context.set_pin_value("maximum", json!(maximum)).await?;
        Ok(())
    }
}
