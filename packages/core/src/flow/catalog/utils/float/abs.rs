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
pub struct AbsFloatNode {}

impl AbsFloatNode {
    pub fn new() -> Self {
        AbsFloatNode {}
    }
}

#[async_trait]
impl NodeLogic for AbsFloatNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "float_abs",
            "Abs",
            "Calculates the absolute value of a float",
            "Math/Float",
        );
        node.add_icon("/flow/icons/sigma.svg");

        node.add_input_pin("float", "Float", "Input Float", VariableType::Float);

        node.add_output_pin(
            "absolute",
            "Absolute",
            "The absolute value of the float",
            VariableType::Float,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        let float: f64 = context.evaluate_pin("float").await?;

        let absolute = float.abs();

        context.set_pin_value("absolute", json!(absolute)).await?;
        Ok(())
    }
}
