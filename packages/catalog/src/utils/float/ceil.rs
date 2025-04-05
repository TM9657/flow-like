use std::{collections::{HashMap, HashSet}, sync::Arc};
use flow_like::{flow::{board::Board, execution::{context::ExecutionContext, internal_node::InternalNode, log::LogMessage, LogLevel}, node::{Node, NodeLogic}, pin::{PinOptions, ValueType}, variable::{Variable, VariableType}}, state::FlowLikeState};
use flow_like_types::{async_trait, json::json, reqwest, sync::{DashMap, Mutex}, Value};

use crate::{storage::path::FlowPath, web::api::{HttpBody, HttpRequest, HttpResponse, Method}};

#[derive(Default)]
pub struct CeilFloatNode {}

impl CeilFloatNode {
    pub fn new() -> Self {
        CeilFloatNode {}
    }
}

#[async_trait]
impl NodeLogic for CeilFloatNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "float_ceil",
            "Ceil",
            "Rounds a float up to the nearest integer",
            "Math/Float",
        );
        node.add_icon("/flow/icons/sigma.svg");

        node.add_input_pin("float", "Float", "Input Float", VariableType::Float);

        node.add_output_pin(
            "ceiling",
            "Ceiling",
            "The ceiling of the float",
            VariableType::Integer,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) ->flow_like_types::Result<()> {
        let float: f64 = context.evaluate_pin("float").await?;

        let ceiling = float.ceil() as i64;

        context.set_pin_value("ceiling", json!(ceiling)).await?;
        Ok(())
    }
}
