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
pub struct FloorFloatNode {}

impl FloorFloatNode {
    pub fn new() -> Self {
        FloorFloatNode {}
    }
}

#[async_trait]
impl NodeLogic for FloorFloatNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "float_floor",
            "Floor",
            "Rounds a float down to the nearest integer",
            "Math/Float",
        );
        node.add_icon("/flow/icons/sigma.svg");

        node.add_input_pin("float", "Float", "Input Float", VariableType::Float);

        node.add_output_pin(
            "floor",
            "Floor",
            "The floor of the float",
            VariableType::Integer,
        );

        return node;
    }

    async fn run(&mut self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        let float: f64 = context.evaluate_pin("float").await?;

        let floor = float.floor() as i64;

        context.set_pin_value("floor", json!(floor)).await?;
        Ok(())
    }
}
