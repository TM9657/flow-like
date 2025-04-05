use flow_like::{
    flow::{
        board::Board,
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        pin::ValueType,
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_types::{async_trait, json::json};
use std::sync::Arc;

#[derive(Default)]
pub struct ArrayLengthNode {}

impl ArrayLengthNode {
    pub fn new() -> Self {
        ArrayLengthNode {}
    }
}

#[async_trait]
impl NodeLogic for ArrayLengthNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "array_length",
            "Array Length",
            "Gets the length of an array",
            "Utils/Array",
        );
        node.add_icon("/flow/icons/grip.svg");

        node.add_input_pin("array", "Array", "Input Array", VariableType::Generic)
            .set_value_type(ValueType::Array);

        node.add_output_pin(
            "length",
            "Length",
            "Length of the array",
            VariableType::Integer,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let array: Vec<flow_like_types::Value> = context.evaluate_pin("array").await?;
        let length = array.len() as i64;
        context.set_pin_value("length", json!(length)).await?;
        Ok(())
    }

    async fn on_update(&self, node: &mut Node, board: Arc<Board>) {
        let _match_type = node.match_type("array", board, Some(ValueType::Array), None);
    }
}
