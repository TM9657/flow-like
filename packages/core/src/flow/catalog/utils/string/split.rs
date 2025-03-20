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
pub struct StringSplitNode {}

impl StringSplitNode {
    pub fn new() -> Self {
        StringSplitNode {}
    }
}

#[async_trait]
impl NodeLogic for StringSplitNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "string_split",
            "Split String",
            "Splits a string into substrings",
            "Utils/String",
        );
        node.add_icon("/flow/icons/string.svg");

        node.add_input_pin("string", "String", "Input String", VariableType::String);
        node.add_input_pin(
            "separator",
            "Separator",
            "String to split by",
            VariableType::String,
        );

        node.add_output_pin(
            "substrings",
            "Substrings",
            "Array of substrings",
            VariableType::String,
        )
        .set_value_type(crate::flow::pin::ValueType::Array);

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        let string: String = context.evaluate_pin("string").await?;
        let separator: String = context.evaluate_pin("separator").await?;

        let substrings: Vec<String> = string.split(&separator).map(|s| s.to_string()).collect();

        context
            .set_pin_value("substrings", json!(substrings))
            .await?;
        Ok(())
    }
}
