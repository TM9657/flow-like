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
use flow_like_types::{Value, async_trait};
use std::sync::Arc;

#[derive(Default)]
pub struct FromBytesNode {}

impl FromBytesNode {
    pub fn new() -> Self {
        FromBytesNode {}
    }
}

#[async_trait]
impl NodeLogic for FromBytesNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "val_from_bytes",
            "From Bytes",
            "Convert String to Bytes",
            "Utils/Conversions",
        );
        node.add_icon("/flow/icons/convert.svg");

        node.add_input_pin("bytes", "Bytes", "Bytes to convert", VariableType::Byte)
            .set_value_type(ValueType::Array);

        node.add_output_pin("value", "Value", "Parsed Value", VariableType::Generic);

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let bytes: Vec<u8> = context.evaluate_pin("bytes").await?;
        let value: Value = flow_like_types::json::from_slice(&bytes)?;
        context.set_pin_value("value", value).await?;
        Ok(())
    }

    async fn on_update(&self, node: &mut Node, board: Arc<Board>) {
        let match_type = node.match_type("value", board, None, None);

        if match_type.is_err() {
            eprintln!("Error: {:?}", match_type.err());
        }
    }
}
