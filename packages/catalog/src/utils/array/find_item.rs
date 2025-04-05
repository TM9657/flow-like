use std::{collections::{HashMap, HashSet}, sync::Arc};
use flow_like::{flow::{board::Board, execution::{context::ExecutionContext, internal_node::InternalNode, log::LogMessage, LogLevel}, node::{Node, NodeLogic}, pin::{PinOptions, ValueType}, variable::{Variable, VariableType}}, state::FlowLikeState};
use flow_like_types::{async_trait, json::json, reqwest, sync::{DashMap, Mutex}, Value};

use crate::{storage::path::FlowPath, web::api::{HttpBody, HttpRequest, HttpResponse, Method}};

#[derive(Default)]
pub struct FindItemInArrayNode {}

impl FindItemInArrayNode {
    pub fn new() -> Self {
        FindItemInArrayNode {}
    }
}

#[async_trait]
impl NodeLogic for FindItemInArrayNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "array_find_item",
            "Find Item",
            "Finds the index of an item in an array",
            "Utils/Array",
        );
        node.add_icon("/flow/icons/grip.svg");

        node.add_input_pin("exec_in", "In", "", VariableType::Execution);

        node.add_input_pin("array_in", "Array", "Your Array", VariableType::Generic)
            .set_value_type(ValueType::Array);

        node.add_input_pin("item", "Item", "Item to find", VariableType::Generic);

        node.add_output_pin("exec_out", "Out", "", VariableType::Execution);
        node.add_output_pin(
            "index",
            "Index",
            "Index of the item (-1 if not found)",
            VariableType::Integer,
        );
        node.add_output_pin(
            "found",
            "Found",
            "Was the item found?",
            VariableType::Boolean,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) ->flow_like_types::Result<()> {
        let array_in: Vec<Value> = context.evaluate_pin("array_in").await?;
        let item: Value = context.evaluate_pin("item").await?;

        let mut index = -1;
        let mut found = false;

        for (i, val) in array_in.iter().enumerate() {
            if val == &item {
                index = i as i64; // Cast to i64 to handle -1
                found = true;
                break;
            }
        }

        context.set_pin_value("index", json!(index)).await?;
        context.set_pin_value("found", json!(found)).await?;
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

        let match_type = node
            .match_type("item", board.clone(), Some(ValueType::Normal), None)
            .unwrap_or(VariableType::Generic);

        if match_type != VariableType::Generic {
            found_type = match_type;
        }

        if found_type != VariableType::Generic {
            node.get_pin_mut_by_name("array_in").unwrap().data_type = found_type.clone();
            node.get_pin_mut_by_name("item").unwrap().data_type = found_type.clone();
        }
    }
}
