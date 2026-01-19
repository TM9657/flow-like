use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic},
    variable::VariableType,
};
use flow_like_types::{async_trait, json::json};

#[crate::register_node]
#[derive(Default)]
pub struct StringLengthNode {}

impl StringLengthNode {
    pub fn new() -> Self {
        StringLengthNode {}
    }
}

#[async_trait]
impl NodeLogic for StringLengthNode {
    fn get_node(&self) -> Node {
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

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let string: String = context.evaluate_pin("string").await?;
        let length = string.len();

        context.set_pin_value("length", json!(length)).await?;
        Ok(())
    }
}
