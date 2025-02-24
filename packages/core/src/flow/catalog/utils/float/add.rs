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
pub struct AddFloatNode {}

impl AddFloatNode {
    pub fn new() -> Self {
        AddFloatNode {}
    }
}

#[async_trait]
impl NodeLogic for AddFloatNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new("float_add", "+", "Adds two floats together", "Math/Float");
        node.add_icon("/flow/icons/sigma.svg");

        node.add_input_pin("float1", "Float 1", "First Float", VariableType::Float);
        node.add_input_pin("float2", "Float 2", "Second Float", VariableType::Float);

        node.add_output_pin(
            "sum",
            "Sum",
            "The sum of the two floats",
            VariableType::Float,
        );

        return node;
    }

    async fn run(&mut self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        let float1: f64 = context.evaluate_pin("float1").await?;
        let float2: f64 = context.evaluate_pin("float2").await?;

        let sum = float1 + float2;

        context.set_pin_value("sum", json!(sum)).await?;
        Ok(())
    }
}
