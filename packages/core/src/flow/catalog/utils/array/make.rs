use std::sync::Arc;

use crate::{
    flow::{
        board::Board,
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        pin::ValueType,
        variable::VariableType,
    },
    state::FlowLikeState,
};
use async_trait::async_trait;
use serde_json::json;

#[derive(Default)]
pub struct MakeArrayNode {}

impl MakeArrayNode {
    pub fn new() -> Self {
        MakeArrayNode {}
    }
}

#[async_trait]
impl NodeLogic for MakeArrayNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "make_array",
            "Make Array",
            "Creates an empty array",
            "Utils/Array",
        );

        node.add_icon("/flow/icons/grip.svg");

        node.add_output_pin(
            "array_out",
            "Array",
            "The created array",
            VariableType::Generic,
        )
        .set_value_type(ValueType::Array);

        return node;
    }

    async fn run(&mut self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        let array_out: Vec<serde_json::Value> = Vec::new(); // Create an empty array
        context.set_pin_value("array_out", json!(array_out)).await?;
        Ok(())
    }

    async fn on_update(&self, node: &mut Node, board: Arc<Board>) {
        let match_type = node
            .match_type("array_out", board, Some(ValueType::Array))
            .unwrap_or(VariableType::Generic);

        if match_type != VariableType::Generic {
            node.get_pin_mut_by_name("array_out").unwrap().data_type = match_type;
        }
    }
}
