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
pub struct UnequalFloatNode {}

impl UnequalFloatNode {
    pub fn new() -> Self {
        UnequalFloatNode {}
    }
}

#[async_trait]
impl NodeLogic for UnequalFloatNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "float_unequal",
            "!=",
            "Checks if two floats are unequal (within a tolerance)",
            "Math/Float/Comparison",
        );
        node.add_icon("/flow/icons/sigma.svg");

        node.add_input_pin("float1", "Float 1", "First Float", VariableType::Float);
        node.add_input_pin("float2", "Float 2", "Second Float", VariableType::Float);
        node.add_input_pin(
            "tolerance",
            "Tolerance",
            "Comparison Tolerance",
            VariableType::Float,
        );

        node.add_output_pin(
            "is_unequal",
            "Is Unequal",
            "True if the floats are unequal, false otherwise",
            VariableType::Boolean,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        let float1: f64 = context.evaluate_pin("float1").await?;
        let float2: f64 = context.evaluate_pin("float2").await?;
        let tolerance: f64 = context.evaluate_pin("tolerance").await?;

        let is_unequal = (float1 - float2).abs() > tolerance;

        context
            .set_pin_value("is_unequal", json!(is_unequal))
            .await?;
        Ok(())
    }
}
