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
pub struct ArrayIncludesNode {}

impl ArrayIncludesNode {
    pub fn new() -> Self {
        ArrayIncludesNode {}
    }
}

#[async_trait]
impl NodeLogic for ArrayIncludesNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "array_includes",
            "Includes",
            "Checks if an array includes a certain value",
            "Utils/Array",
        );

        node.add_icon("/flow/icons/grip.svg");

        node.add_input_pin("array_in", "Array", "Your Array", VariableType::Generic)
            .set_value_type(ValueType::Array);

        node.add_input_pin(
            "value",
            "Value",
            "Value to search for",
            VariableType::Generic,
        );

        node.add_output_pin(
            "includes",
            "Includes?",
            "Does the array include the value?",
            VariableType::Boolean,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        let array_in: Vec<Value> = context.evaluate_pin("array_in").await?;
        let value: Value = context.evaluate_pin("value").await?;

        let includes = array_in.contains(&value);

        context.set_pin_value("includes", json!(includes)).await?;
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

        let match_type = node
            .match_type("value", board, Some(ValueType::Normal), None)
            .unwrap_or(VariableType::Generic);

        if match_type != VariableType::Generic {
            found_type = match_type;
        }

        if found_type != VariableType::Generic {
            node.get_pin_mut_by_name("array_in").unwrap().data_type = found_type.clone();
            node.get_pin_mut_by_name("value").unwrap().data_type = found_type;
        }
    }
}
