use std::{collections::{HashMap, HashSet}, sync::Arc};
use flow_like::{flow::{board::Board, execution::{context::ExecutionContext, internal_node::InternalNode, log::LogMessage, LogLevel}, node::{Node, NodeLogic}, pin::{PinOptions, ValueType}, variable::{Variable, VariableType}}, state::FlowLikeState};
use flow_like_types::{async_trait, json::json, reqwest, sync::{DashMap, Mutex}, Value};

use crate::{storage::path::FlowPath, web::api::{HttpBody, HttpRequest, HttpResponse, Method}};

#[derive(Default)]
pub struct UnequalIntegerNode {}

impl UnequalIntegerNode {
    pub fn new() -> Self {
        UnequalIntegerNode {}
    }
}

#[async_trait]
impl NodeLogic for UnequalIntegerNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "int_unequal",
            "!=",
            "Checks if two integers are unequal",
            "Math/Int",
        );
        node.add_icon("/flow/icons/sigma.svg");

        node.add_input_pin(
            "integer1",
            "Integer 1",
            "Input Integer",
            VariableType::Integer,
        );
        node.add_input_pin(
            "integer2",
            "Integer 2",
            "Input Integer",
            VariableType::Integer,
        );

        node.add_output_pin(
            "unequal",
            "Unequal",
            "True if the integers are unequal, false otherwise",
            VariableType::Boolean,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) ->flow_like_types::Result<()> {
        let integer1: i64 = context.evaluate_pin("integer1").await?;
        let integer2: i64 = context.evaluate_pin("integer2").await?;

        let unequal = integer1 != integer2;

        context.set_pin_value("unequal", json!(unequal)).await?;
        Ok(())
    }
}
