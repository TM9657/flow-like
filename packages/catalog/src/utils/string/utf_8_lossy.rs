use flow_like::{
    flow::{
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        pin::ValueType,
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_types::{async_trait, json::json};

#[derive(Default)]
pub struct ParseUtf8LossyNode {}

impl ParseUtf8LossyNode {
    pub fn new() -> Self {
        ParseUtf8LossyNode {}
    }
}

#[async_trait]
impl NodeLogic for ParseUtf8LossyNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "utf8_lossy",
            "From UTF-8 Lossy",
            "Converts a byte array to a string using the UTF-8 lossy strategy",
            "Utils/String",
        );
        node.add_icon("/flow/icons/string.svg");

        node.add_input_pin("bytes", "Bytes", "", VariableType::Byte)
            .set_value_type(ValueType::Array);
        node.add_output_pin("string", "String", "Input String", VariableType::String);

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let bytes: Vec<u8> = context.evaluate_pin("bytes").await?;
        let string = String::from_utf8_lossy(&bytes).to_string();

        context.set_pin_value("string", json!(string)).await?;
        Ok(())
    }
}
