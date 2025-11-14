use flow_like::{
    flow::{
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_types::{async_trait, json::json};

#[crate::register_node]
#[derive(Default)]
pub struct StringReplaceNode {}

impl StringReplaceNode {
    pub fn new() -> Self {
        StringReplaceNode {}
    }
}

#[async_trait]
impl NodeLogic for StringReplaceNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "string_replace",
            "Replace String",
            "Replaces occurrences of a substring within a string.",
            "Utils/String",
        );
        node.add_icon("/flow/icons/string.svg");

        node.add_input_pin("string", "String", "Input String", VariableType::String);
        node.add_input_pin(
            "pattern",
            "Pattern",
            "Substring to replace",
            VariableType::String,
        );
        node.add_input_pin(
            "replacement",
            "Replacement",
            "Replacement string",
            VariableType::String,
        );

        node.add_output_pin(
            "new_string",
            "New String",
            "String with replacements",
            VariableType::String,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let string: String = context.evaluate_pin("string").await?;
        let pattern: String = context.evaluate_pin("pattern").await?;
        let replacement: String = context.evaluate_pin("replacement").await?;

        let new_string = string.replace(&pattern, &replacement);

        context
            .set_pin_value("new_string", json!(new_string))
            .await?;
        Ok(())
    }
}
