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
pub struct StringToLowerNode {}

impl StringToLowerNode {
    pub fn new() -> Self {
        StringToLowerNode {}
    }
}

#[async_trait]
impl NodeLogic for StringToLowerNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "string_to_lower",
            "To Lower Case",
            "Converts a string to lowercase",
            "Utils/String",
        );
        node.add_icon("/flow/icons/string.svg");

        node.add_input_pin("string", "String", "Input String", VariableType::String);

        node.add_output_pin(
            "lowercase_string",
            "Lowercase String",
            "String in lowercase",
            VariableType::String,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        let string: String = context.evaluate_pin("string").await?;
        let lowercase_string = string.to_lowercase();

        context
            .set_pin_value("lowercase_string", json!(lowercase_string))
            .await?;
        Ok(())
    }
}
