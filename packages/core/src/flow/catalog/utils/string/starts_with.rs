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
pub struct StringStartsWithNode {}

impl StringStartsWithNode {
    pub fn new() -> Self {
        StringStartsWithNode {}
    }
}

#[async_trait]
impl NodeLogic for StringStartsWithNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "string_starts_with",
            "Starts With",
            "Checks if a string starts with a specific string",
            "Utils/String",
        );
        node.add_icon("/flow/icons/string.svg");

        node.add_input_pin("string", "String", "Input String", VariableType::String);
        node.add_input_pin(
            "prefix",
            "Prefix",
            "String to check against",
            VariableType::String,
        );

        node.add_output_pin(
            "starts_with",
            "Starts With?",
            "Does the string start with the prefix?",
            VariableType::Boolean,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        let string: String = context.evaluate_pin("string").await?;
        let prefix: String = context.evaluate_pin("prefix").await?;

        let starts_with = string.starts_with(&prefix);

        context
            .set_pin_value("starts_with", json!(starts_with))
            .await?;
        Ok(())
    }
}
