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
pub struct GreaterThanOrEqualIntegerNode {}

impl GreaterThanOrEqualIntegerNode {
    pub fn new() -> Self {
        GreaterThanOrEqualIntegerNode {}
    }
}

#[async_trait]
impl NodeLogic for GreaterThanOrEqualIntegerNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "int_greater_than_or_equal",
            ">=",
            "Checks if the first integer is greater than or equal to the second",
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
            "greater_than_or_equal",
            "Greater Than or Equal",
            "True if integer1 >= integer2, false otherwise",
            VariableType::Boolean,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        let integer1: i64 = context.evaluate_pin("integer1").await?;
        let integer2: i64 = context.evaluate_pin("integer2").await?;

        let greater_than_or_equal = integer1 >= integer2;

        context
            .set_pin_value("greater_than_or_equal", json!(greater_than_or_equal))
            .await?;
        Ok(())
    }
}
