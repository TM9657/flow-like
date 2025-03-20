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
pub struct SubtractFloatNode {}

impl SubtractFloatNode {
    pub fn new() -> Self {
        SubtractFloatNode {}
    }
}

#[async_trait]
impl NodeLogic for SubtractFloatNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "float_subtract",
            "-",
            "Subtracts one float from another",
            "Math/Float",
        );
        node.add_icon("/flow/icons/sigma.svg");

        node.add_input_pin("float1", "Float 1", "First Float", VariableType::Float);
        node.add_input_pin("float2", "Float 2", "Second Float", VariableType::Float);

        node.add_output_pin(
            "difference",
            "Difference",
            "The difference between the two floats",
            VariableType::Float,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        let float1: f64 = context.evaluate_pin("float1").await?;
        let float2: f64 = context.evaluate_pin("float2").await?;

        let difference = float1 - float2;

        context
            .set_pin_value("difference", json!(difference))
            .await?;
        Ok(())
    }
}
