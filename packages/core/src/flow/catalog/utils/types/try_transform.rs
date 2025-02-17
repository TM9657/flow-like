use std::sync::Arc;

use crate::{
    flow::{
        board::Board,
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        variable::VariableType,
    },
    state::FlowLikeState,
};
use async_trait::async_trait;

#[derive(Default)]

pub struct TryTransformNode {}

impl TryTransformNode {
    pub fn new() -> Self {
        TryTransformNode {}
    }
}

#[async_trait]
impl NodeLogic for TryTransformNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "utils_types_try_transform",
            "Try Transform",
            "Tries to transform cast types.",
            "Utils/Types",
        );

        node.add_input_pin(
            "type_in",
            "Type In",
            "Type to transform",
            VariableType::Generic,
        );

        node.add_output_pin(
            "type_out",
            "Type Out",
            "If the type was successfully transformed, transformed type",
            VariableType::Generic,
        );

        node.add_output_pin(
            "success",
            "Success",
            "Determines of tje transformation was successful",
            VariableType::Boolean,
        );

        return node;
    }

    async fn run(&mut self, _context: &mut ExecutionContext) -> anyhow::Result<()> {
        return Ok(());
    }

    async fn on_update(&self, node: &mut Node, board: Arc<Board>) {
        let match_type = node.match_type("type_out", board.clone(), None);

        if match_type.is_err() {
            eprintln!("Error: {:?}", match_type.err());
        }

        let match_type = node.match_type("type_in", board, None);

        if match_type.is_err() {
            eprintln!("Error: {:?}", match_type.err());
        }
    }
}
