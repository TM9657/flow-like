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
pub struct SubtractIntegerNode {}

impl SubtractIntegerNode {
    pub fn new() -> Self {
        SubtractIntegerNode {}
    }
}

#[async_trait]
impl NodeLogic for SubtractIntegerNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new("int_subtract", "-", "Subtracts two Integers", "Math/Int");
        node.add_icon("/flow/icons/sigma.svg");

        node.add_input_pin("integer1", "Integer 1", "Minuend", VariableType::Integer);
        node.add_input_pin("integer2", "Integer 2", "Subtrahend", VariableType::Integer);

        node.add_output_pin(
            "difference",
            "Difference",
            "Difference of the two integers",
            VariableType::Integer,
        );

        return node;
    }

    async fn run(&mut self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        let integer1: i64 = context.evaluate_pin("integer1").await?;
        let integer2: i64 = context.evaluate_pin("integer2").await?;
        let difference = integer1 - integer2;
        context
            .set_pin_value("difference", json!(difference))
            .await?;
        Ok(())
    }
}
