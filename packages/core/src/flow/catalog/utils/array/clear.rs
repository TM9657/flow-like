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
use serde_json::{Value, json};

#[derive(Default)]
pub struct ClearArrayNode {}

impl ClearArrayNode {
    pub fn new() -> Self {
        ClearArrayNode {}
    }
}

#[async_trait]
impl NodeLogic for ClearArrayNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "array_clear",
            "Clear Array",
            "Removes all elements from an array",
            "Utils/Array",
        );

        node.add_icon("/flow/icons/grip.svg");

        node.add_input_pin("exec_in", "In", "", VariableType::Execution);

        node.add_input_pin("array_in", "Array", "Your Array", VariableType::Generic)
            .set_value_type(crate::flow::pin::ValueType::Array);

        node.add_output_pin("exec_out", "Out", "", VariableType::Execution);

        node.add_output_pin("array_out", "Array", "Empty Array", VariableType::Generic)
            .set_value_type(crate::flow::pin::ValueType::Array);

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        let _array_in: Vec<Value> = context.evaluate_pin("array_in").await?; // We read it to keep the pin active, but don't need it
        let array_out: Vec<Value> = Vec::new();

        context.set_pin_value("array_out", json!(array_out)).await?;
        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }

    async fn on_update(&self, node: &mut Node, board: Arc<Board>) {
        let mut found_type = VariableType::Generic;
        let match_type = node
            .match_type("array_in", board.clone(), Some(ValueType::Array), None)
            .unwrap_or(VariableType::Generic);

        if match_type != VariableType::Generic {
            found_type = match_type;
        }

        if found_type != VariableType::Generic {
            node.get_pin_mut_by_name("array_out").unwrap().data_type = found_type.clone();
            node.get_pin_mut_by_name("array_in").unwrap().data_type = found_type.clone();
        }
    }
}
