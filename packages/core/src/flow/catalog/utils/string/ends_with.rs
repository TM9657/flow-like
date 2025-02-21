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
pub struct StringEndsWithNode {}

impl StringEndsWithNode {
    pub fn new() -> Self {
        StringEndsWithNode {}
    }
}

#[async_trait]
impl NodeLogic for StringEndsWithNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "string_ends_with",
            "Ends With",
            "Checks if a string ends with a specific string",
            "Utils/String",
        );
        node.add_icon("/flow/icons/string.svg");

        node.add_input_pin("string", "String", "Input String", VariableType::String);
        node.add_input_pin(
            "suffix",
            "Suffix",
            "String to check against",
            VariableType::String,
        );

        node.add_output_pin(
            "ends_with",
            "Ends With?",
            "Does the string end with the suffix?",
            VariableType::Boolean,
        );

        return node;
    }

    async fn run(&mut self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        let string: String = context.evaluate_pin("string").await?;
        let suffix: String = context.evaluate_pin("suffix").await?;

        let ends_with = string.ends_with(&suffix);

        context.set_pin_value("ends_with", json!(ends_with)).await?;
        Ok(())
    }
}
