use std::{collections::{HashMap, HashSet}, sync::Arc};
use flow_like::{flow::{board::Board, execution::{context::ExecutionContext, internal_node::InternalNode, log::LogMessage, LogLevel}, node::{Node, NodeLogic}, pin::{PinOptions, ValueType}, variable::{Variable, VariableType}}, state::FlowLikeState};
use flow_like_types::{async_trait, json::json, reqwest, sync::{DashMap, Mutex}, Value};

use crate::{storage::path::FlowPath, web::api::{HttpBody, HttpRequest, HttpResponse, Method}};

#[derive(Default)]
pub struct RemoveArrayIndexNode {}

impl RemoveArrayIndexNode {
    pub fn new() -> Self {
        RemoveArrayIndexNode {}
    }
}

#[async_trait]
impl NodeLogic for RemoveArrayIndexNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "array_remove_index",
            "Remove Index",
            "Removes an element from an array at a specific index",
            "Utils/Array",
        );
        node.add_icon("/flow/icons/grip.svg");

        node.add_input_pin("exec_in", "In", "", VariableType::Execution);

        node.add_input_pin("array_in", "Array", "Your Array", VariableType::Generic)
            .set_value_type(ValueType::Array);

        node.add_input_pin("index", "Index", "Index to remove", VariableType::Integer);

        node.add_output_pin("exec_out", "Out", "", VariableType::Execution);

        node.add_output_pin(
            "array_out",
            "Array",
            "Adjusted Array",
            VariableType::Generic,
        )
        .set_value_type(ValueType::Array);

        node.add_output_pin(
            "failed",
            "Failed Removal",
            "Triggered if the Removal failed",
            VariableType::Execution,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) ->flow_like_types::Result<()> {
        context.activate_exec_pin("failed").await?;
        context.deactivate_exec_pin("exec_out").await?;

        let array_in: Vec<Value> = context.evaluate_pin("array_in").await?;
        let index: usize = context.evaluate_pin("index").await?;

        let mut array_out = array_in.clone();
        let success = index < array_out.len();

        if success {
            array_out.remove(index);
        } else {
            context.log_message(
                &format!(
                    "Index {} is out of bounds for array of length {}",
                    index,
                    array_out.len()
                ),
                LogLevel::Warn,
            );
        }

        context.set_pin_value("array_out", json!(array_out)).await?;

        if success {
            context.deactivate_exec_pin("failed").await?;
            context.activate_exec_pin("exec_out").await?;
        }

        Ok(())
    }

    async fn on_update(&self, node: &mut Node, board: Arc<Board>) {
        let mut found_type = VariableType::Generic;
        let match_type = node
            .match_type("array_out", board.clone(), Some(ValueType::Array), None)
            .unwrap_or(VariableType::Generic);

        if match_type != VariableType::Generic {
            found_type = match_type;
        }

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
