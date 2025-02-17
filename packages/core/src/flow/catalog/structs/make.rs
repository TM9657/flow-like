use std::collections::HashMap;

use crate::{
    flow::{
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        variable::VariableType,
    },
    state::FlowLikeState,
};
use async_trait::async_trait;

#[derive(Default)]
pub struct MakeStructNode {}

impl MakeStructNode {
    pub fn new() -> Self {
        MakeStructNode {}
    }
}

#[async_trait]
impl NodeLogic for MakeStructNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "struct_make",
            "Make Struct",
            "Creates a new struct",
            "Structs",
        );
        node.add_icon("/flow/icons/struct.svg");

        node.add_output_pin("struct", "Struct", "Struct Output", VariableType::Struct);

        return node;
    }

    async fn run(&mut self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        let empty_struct: HashMap<String, serde_json::Value> = HashMap::new();
        context
            .set_pin_value("struct", serde_json::json!(empty_struct))
            .await?;

        return Ok(());
    }
}
