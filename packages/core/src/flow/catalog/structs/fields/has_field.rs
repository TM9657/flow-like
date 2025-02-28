use crate::{
    flow::{
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        variable::VariableType,
    },
    state::FlowLikeState,
};
use ahash::HashMap;
use async_trait::async_trait;

#[derive(Default)]
pub struct HasStructFieldNode {}

impl HasStructFieldNode {
    pub fn new() -> Self {
        HasStructFieldNode {}
    }
}

#[async_trait]
impl NodeLogic for HasStructFieldNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "struct_has",
            "Has Field",
            "Checks if a field exists in a struct",
            "Structs/Fields",
        );
        node.add_icon("/flow/icons/struct.svg");

        node.add_output_pin(
            "found",
            "Found?",
            "Indicates if the value was found",
            VariableType::Boolean,
        );

        node.add_input_pin("struct", "Struct", "Struct Output", VariableType::Struct);

        node.add_input_pin("field", "Field", "Field to get", VariableType::String);

        return node;
    }

    async fn run(&mut self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        let struct_value = context
            .evaluate_pin::<HashMap<String, serde_json::Value>>("struct")
            .await?;
        let field = context.evaluate_pin::<String>("field").await?;

        let value = struct_value.get(&field);
        context
            .set_pin_value("found", serde_json::json!(value.is_some()))
            .await?;

        return Ok(());
    }
}
