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
use serde_json::{json, Value};

#[derive(Default)]
pub struct SetIndexArrayNode {}

impl SetIndexArrayNode {
    pub fn new() -> Self {
        SetIndexArrayNode {}
    }
}

#[async_trait]
impl NodeLogic for SetIndexArrayNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "array_set_index",
            "Set Index",
            "Sets an element at a specific index in an array",
            "Utils/Array",
        );
        node.add_icon("/flow/icons/array.svg");

        node.add_input_pin("exec_in", "In", "", VariableType::Execution);

        node.add_input_pin("array_in", "Array", "Your Array", VariableType::Generic)
            .set_value_type(ValueType::Array);

        node.add_input_pin("index", "Index", "Index to set", VariableType::Integer);

        node.add_input_pin("value", "Value", "Value to set", VariableType::Generic);

        node.add_output_pin("exec_out", "Out", "", VariableType::Execution);

        node.add_output_pin(
            "array_out",
            "Array",
            "Adjusted Array",
            VariableType::Generic,
        )
        .set_value_type(ValueType::Array);

        node.add_output_pin(
            "success",
            "Success",
            "Was the operation successful?",
            VariableType::Boolean,
        );

        return node;
    }

    async fn run(&mut self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        let array_in: Vec<Value> = context.evaluate_pin("array_in").await?;
        let index: usize = context.evaluate_pin("index").await?;
        let value: Value = context.evaluate_pin("value").await?;

        let mut array_out = array_in.clone();
        let mut success = false;

        if index < array_out.len() {
            success = true;
            array_out[index] = value;
        }

        context.set_pin_value("array_out", json!(array_out)).await?;
        context.set_pin_value("success", json!(success)).await?;
        context.activate_exec_pin("exec_out").await?;
        Ok(())
    }

    async fn on_update(&self, node: &mut Node, board: Arc<Board>) {
        let mut found_type = VariableType::Generic;
        let match_type = node
            .match_type("array_out", board.clone(), Some(ValueType::Array))
            .unwrap_or(VariableType::Generic);

        if match_type != VariableType::Generic {
            found_type = match_type;
        }

        let match_type = node
            .match_type("array_in", board.clone(), Some(ValueType::Array))
            .unwrap_or(VariableType::Generic);

        if match_type != VariableType::Generic {
            found_type = match_type;
        }

        let match_type = node
            .match_type("value", board, Some(ValueType::Normal))
            .unwrap_or(VariableType::Generic);

        if match_type != VariableType::Generic {
            found_type = match_type;
        }

        if found_type != VariableType::Generic {
            node.get_pin_mut_by_name("array_out").unwrap().data_type = found_type.clone();
            node.get_pin_mut_by_name("array_in").unwrap().data_type = found_type.clone();
            node.get_pin_mut_by_name("value").unwrap().data_type = found_type;
        }
    }
}
