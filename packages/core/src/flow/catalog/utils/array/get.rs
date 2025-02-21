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
pub struct GetArrayElementNode {}

impl GetArrayElementNode {
    pub fn new() -> Self {
        GetArrayElementNode {}
    }
}

#[async_trait]
impl NodeLogic for GetArrayElementNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "array_get",
            "Get Element",
            "Gets an element from an array by index",
            "Utils/Array",
        );

        node.add_icon("/flow/icons/grip.svg");

        node.add_input_pin("array_in", "Array", "Your Array", VariableType::Generic)
            .set_value_type(ValueType::Array);

        node.add_input_pin(
            "index",
            "Index",
            "Index of the element to get",
            VariableType::Integer,
        );

        node.add_output_pin(
            "element",
            "Element",
            "Element at the specified index",
            VariableType::Generic,
        );

        node.add_output_pin(
            "success",
            "Success",
            "Was the get successful?",
            VariableType::Boolean,
        );

        return node;
    }

    async fn run(&mut self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        let array_in: Vec<Value> = context.evaluate_pin("array_in").await?;
        let index: i64 = context.evaluate_pin("index").await?;

        let mut success = false;
        let mut element = Value::Null;

        if index >= 0 && index < array_in.len() as i64 {
            element = array_in[index as usize].clone();
            success = true;
        }

        context.set_pin_value("element", json!(element)).await?;
        context.set_pin_value("success", json!(success)).await?;
        Ok(())
    }

    async fn on_update(&self, node: &mut Node, board: Arc<Board>) {
        let mut found_type = VariableType::Generic;
        let match_type = node
            .match_type("array_in", board.clone(), Some(ValueType::Array))
            .unwrap_or(VariableType::Generic);

        if match_type != VariableType::Generic {
            found_type = match_type;
        }

        let match_type = node
            .match_type("element", board, Some(ValueType::Normal))
            .unwrap_or(VariableType::Generic);

        if match_type != VariableType::Generic {
            found_type = match_type;
        }

        if found_type != VariableType::Generic {
            node.get_pin_mut_by_name("array_in").unwrap().data_type = found_type.clone();
            node.get_pin_mut_by_name("element").unwrap().data_type = found_type;
        }
    }
}
