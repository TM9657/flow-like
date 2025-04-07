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
use flow_like_types::{Value, async_trait, json::from_str};
use std::sync::Arc;

#[derive(Default)]
pub struct FromStringNode {}

impl FromStringNode {
    pub fn new() -> Self {
        FromStringNode {}
    }
}

#[async_trait]
impl NodeLogic for FromStringNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "val_from_string",
            "From String",
            "Convert String to Struct",
            "Utils/Conversions",
        );
        node.add_icon("/flow/icons/convert.svg");

        node.add_input_pin(
            "string",
            "String",
            "String to convert",
            VariableType::String,
        );

        node.add_output_pin(
            "value_ref",
            "Value",
            "Value of the Generic",
            VariableType::Generic,
        );
        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let string: String = context.evaluate_pin("string").await?;
        let value: Value = from_str(&string)?;
        context.set_pin_value("value_ref", value).await?;
        Ok(())
    }

    async fn on_update(&self, node: &mut Node, board: Arc<Board>) {
        let match_type = node.match_type("value_ref", board, None, None);

        if match_type.is_err() {
            eprintln!("Error: {:?}", match_type.err());
        }
    }
}
