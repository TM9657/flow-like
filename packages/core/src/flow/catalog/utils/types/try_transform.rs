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
        let pin = match node.get_pin_by_name("type_out") {
            Some(pin) => pin,
            None => {
                println!("Pin not found");
                return;
            }
        };

        let connected = pin.connected_to.clone();

        let pin = match node.get_pin_by_name("type_in") {
            Some(pin) => pin,
            None => {
                println!("Pin not found");
                return;
            }
        };

        let dependent = pin.depends_on.clone();

        node.get_pin_mut_by_name("type_in").unwrap().data_type = VariableType::Generic;
        node.get_pin_mut_by_name("type_out").unwrap().data_type = VariableType::Generic;

        if let Some(first_dependent) = dependent.iter().next() {
            let pin = board.get_pin_by_id(first_dependent);
            let mutable_pin = node.get_pin_mut_by_name("type_in").unwrap();

            match pin {
                Some(pin) => {
                    mutable_pin.data_type = pin.data_type.clone();
                    mutable_pin.value_type = pin.value_type.clone();
                }
                None => {
                    mutable_pin.depends_on.remove(first_dependent);
                }
            }
        }

        if let Some(first_connected) = connected.iter().next() {
            let pin = board.get_pin_by_id(first_connected);
            let mutable_pin = node.get_pin_mut_by_name("type_out").unwrap();

            match pin {
                Some(pin) => {
                    mutable_pin.data_type = pin.data_type.clone();
                    mutable_pin.value_type = pin.value_type.clone();
                }
                None => {
                    mutable_pin.connected_to.remove(first_connected);
                }
            }
        }
    }
}
