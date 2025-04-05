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
pub struct EqualFloatNode {}

impl EqualFloatNode {
    pub fn new() -> Self {
        EqualFloatNode {}
    }
}

#[async_trait]
impl NodeLogic for EqualFloatNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "float_equal",
            "==",
            "Checks if two floats are equal (within a tolerance)",
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
            "is_equal",
            "Is Equal",
            "True if the floats are equal, false otherwise",
            VariableType::Boolean,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let float1: f64 = context.evaluate_pin("float1").await?;
        let float2: f64 = context.evaluate_pin("float2").await?;
        let tolerance: f64 = context.evaluate_pin("tolerance").await?;

        let is_equal = (float1 - float2).abs() <= tolerance;

        context.set_pin_value("is_equal", json!(is_equal)).await?;
        Ok(())
    }
}
