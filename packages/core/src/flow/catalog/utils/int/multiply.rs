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
pub struct MultiplyIntegerNode {}

impl MultiplyIntegerNode {
    pub fn new() -> Self {
        MultiplyIntegerNode {}
    }
}

#[async_trait]
impl NodeLogic for MultiplyIntegerNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new("int_multiply", "*", "Multiplies two Integers", "Math/Int");
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
            "product",
            "Product",
            "Product of the two integers",
            VariableType::Integer,
        );

        return node;
    }

    async fn run(&mut self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        let integer1: i64 = context.evaluate_pin("integer1").await?;
        let integer2: i64 = context.evaluate_pin("integer2").await?;
        let product = integer1 * integer2;
        context.set_pin_value("product", json!(product)).await?;
        Ok(())
    }
}
