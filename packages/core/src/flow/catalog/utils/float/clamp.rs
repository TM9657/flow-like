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
pub struct ClampFloatNode {}

impl ClampFloatNode {
    pub fn new() -> Self {
        ClampFloatNode {}
    }
}

#[async_trait]
impl NodeLogic for ClampFloatNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "float_clamp",
            "Clamp",
            "Clamps a float within a given range",
            "Math/Float",
        );
        node.add_icon("/flow/icons/sigma.svg");

        node.add_input_pin("float", "Float", "Input Float", VariableType::Float);
        node.add_input_pin("min", "Min", "Minimum Value", VariableType::Float);
        node.add_input_pin("max", "Max", "Maximum Value", VariableType::Float);

        node.add_output_pin(
            "clamped",
            "Clamped",
            "The clamped float",
            VariableType::Float,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        let float: f64 = context.evaluate_pin("float").await?;
        let min: f64 = context.evaluate_pin("min").await?;
        let max: f64 = context.evaluate_pin("max").await?;

        let clamped = float.clamp(min, max);

        context.set_pin_value("clamped", json!(clamped)).await?;
        Ok(())
    }
}
