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
pub struct LessThanIntegerNode {}

impl LessThanIntegerNode {
    pub fn new() -> Self {
        LessThanIntegerNode {}
    }
}

#[async_trait]
impl NodeLogic for LessThanIntegerNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "int_less_than",
            "<",
            "Checks if the first integer is less than the second",
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
            "less_than",
            "Less Than",
            "True if integer1 < integer2, false otherwise",
            VariableType::Boolean,
        );

        return node;
    }

    async fn run(&mut self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        let integer1: i64 = context.evaluate_pin("integer1").await?;
        let integer2: i64 = context.evaluate_pin("integer2").await?;

        let less_than = integer1 < integer2;

        context.set_pin_value("less_than", json!(less_than)).await?;
        Ok(())
    }
}
