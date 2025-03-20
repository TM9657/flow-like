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
pub struct StringJoinNode {}

impl StringJoinNode {
    pub fn new() -> Self {
        StringJoinNode {}
    }
}

#[async_trait]
impl NodeLogic for StringJoinNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "string_join",
            "Join Strings",
            "Joins multiple strings together",
            "Utils/String",
        );
        node.add_icon("/flow/icons/string.svg");

        node.add_input_pin(
            "strings",
            "Strings",
            "Strings to join",
            VariableType::String,
        )
        .set_value_type(crate::flow::pin::ValueType::Array);
        node.add_input_pin(
            "separator",
            "Separator",
            "String to separate by",
            VariableType::String,
        );

        node.add_output_pin(
            "joined_string",
            "Joined String",
            "Concatenated string",
            VariableType::String,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        let strings: Vec<String> = context.evaluate_pin("strings").await?;
        let separator: String = context.evaluate_pin("separator").await?;

        let joined_string = strings.join(&separator);

        context
            .set_pin_value("joined_string", json!(joined_string))
            .await?;
        Ok(())
    }
}
