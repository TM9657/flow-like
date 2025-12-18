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
pub struct StringContainsNode {}

impl StringContainsNode {
    pub fn new() -> Self {
        StringContainsNode {}
    }
}

#[async_trait]
impl NodeLogic for StringContainsNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "string_contains",
            "Contains",
            "Checks if a string contains a substring",
            "Utils/String",
        );
        node.add_icon("/flow/icons/string.svg");

        node.add_input_pin("string", "String", "Input String", VariableType::String);
        node.add_input_pin(
            "substring",
            "Substring",
            "Substring to search for",
            VariableType::String,
        );

        node.add_output_pin(
            "contains",
            "Contains?",
            "Does the string contain the substring?",
            VariableType::Boolean,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let string = context.evaluate_pin_to_ref("string").await?;
        let substring: String = context.evaluate_pin("substring").await?;

        let mut contains = false;

        {
            let string = string.as_ref().lock().await;
            if let Some(string) = string.as_str() {
                contains = string.contains(&substring);
            }
        }

        context.set_pin_value("contains", json!(contains)).await?;
        Ok(())
    }
}
