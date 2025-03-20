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
pub struct StringLengthNode {}

impl StringLengthNode {
    pub fn new() -> Self {
        StringLengthNode {}
    }
}

#[async_trait]
impl NodeLogic for StringLengthNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "string_length",
            "String Length",
            "Calculates the length of a string",
            "Utils/String",
        );
        node.add_icon("/flow/icons/string.svg");

        node.add_input_pin("string", "String", "Input String", VariableType::String);

        node.add_output_pin(
            "length",
            "Length",
            "Length of the string",
            VariableType::Integer,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        let string: String = context.evaluate_pin("string").await?;
        let length = string.len();

        context.set_pin_value("length", json!(length)).await?;
        Ok(())
    }
}
