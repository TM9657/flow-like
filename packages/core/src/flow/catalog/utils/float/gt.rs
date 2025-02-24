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
pub struct GreaterThanFloatNode {}

impl GreaterThanFloatNode {
    pub fn new() -> Self {
        GreaterThanFloatNode {}
    }
}

#[async_trait]
impl NodeLogic for GreaterThanFloatNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "float_greater_than",
            ">",
            "Checks if one float is greater than another",
            "Math/Float/Comparison",
        );
        node.add_icon("/flow/icons/sigma.svg");

        node.add_input_pin("float1", "Float 1", "First Float", VariableType::Float);
        node.add_input_pin("float2", "Float 2", "Second Float", VariableType::Float);

        node.add_output_pin(
            "is_greater",
            "Is Greater",
            "True if float1 is greater than float2, false otherwise",
            VariableType::Boolean,
        );

        return node;
    }

    async fn run(&mut self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        let float1: f64 = context.evaluate_pin("float1").await?;
        let float2: f64 = context.evaluate_pin("float2").await?;

        let is_greater = float1 > float2;

        context
            .set_pin_value("is_greater", json!(is_greater))
            .await?;
        Ok(())
    }
}
