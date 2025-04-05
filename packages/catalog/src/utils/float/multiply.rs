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
pub struct MultiplyFloatNode {}

impl MultiplyFloatNode {
    pub fn new() -> Self {
        MultiplyFloatNode {}
    }
}

#[async_trait]
impl NodeLogic for MultiplyFloatNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "float_multiply",
            "*",
            "Multiplies two floats together",
            "Math/Float",
        );
        node.add_icon("/flow/icons/sigma.svg");

        node.add_input_pin("float1", "Float 1", "First Float", VariableType::Float);
        node.add_input_pin("float2", "Float 2", "Second Float", VariableType::Float);

        node.add_output_pin(
            "product",
            "Product",
            "The product of the two floats",
            VariableType::Float,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let float1: f64 = context.evaluate_pin("float1").await?;
        let float2: f64 = context.evaluate_pin("float2").await?;

        let product = float1 * float2;

        context.set_pin_value("product", json!(product)).await?;
        Ok(())
    }
}
