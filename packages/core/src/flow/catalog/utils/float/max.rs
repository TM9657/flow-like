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
pub struct MaxFloatNode {}

impl MaxFloatNode {
    pub fn new() -> Self {
        MaxFloatNode {}
    }
}

#[async_trait]
impl NodeLogic for MaxFloatNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "float_max",
            "Max",
            "Returns the larger of two floats",
            "Math/Float",
        );
        node.add_icon("/flow/icons/sigma.svg");

        node.add_input_pin("float1", "Float 1", "First Float", VariableType::Float);
        node.add_input_pin("float2", "Float 2", "Second Float", VariableType::Float);

        node.add_output_pin(
            "maximum",
            "Maximum",
            "The larger of the two floats",
            VariableType::Float,
        );

        return node;
    }

    async fn run(&mut self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        let float1: f64 = context.evaluate_pin("float1").await?;
        let float2: f64 = context.evaluate_pin("float2").await?;

        let maximum = float1.max(float2);

        context.set_pin_value("maximum", json!(maximum)).await?;
        Ok(())
    }
}
