use flow_like::{
    flow::{
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_types::{async_trait, json::json};

#[derive(Default)]
pub struct RoundFloatNode {}

impl RoundFloatNode {
    pub fn new() -> Self {
        RoundFloatNode {}
    }
}

#[async_trait]
impl NodeLogic for RoundFloatNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "float_round",
            "Round",
            "Rounds a float to the nearest integer",
            "Math/Float",
        );
        node.add_icon("/flow/icons/sigma.svg");

        node.add_input_pin("float", "Float", "Input Float", VariableType::Float);

        node.add_output_pin(
            "rounded",
            "Rounded",
            "The rounded float",
            VariableType::Float,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let float: f64 = context.evaluate_pin("float").await?;

        let rounded = float.round();

        context.set_pin_value("rounded", json!(rounded)).await?;
        Ok(())
    }
}
