use std::{collections::{HashMap, HashSet}, sync::Arc};
use flow_like::{flow::{board::Board, execution::{context::ExecutionContext, internal_node::InternalNode, log::LogMessage, LogLevel}, node::{Node, NodeLogic}, pin::{PinOptions, ValueType}, variable::{Variable, VariableType}}, state::FlowLikeState};
use flow_like_types::{async_trait, json::json, reqwest, sync::{DashMap, Mutex}, Value};

use crate::{storage::path::FlowPath, web::api::{HttpBody, HttpRequest, HttpResponse, Method}};

#[derive(Default)]
pub struct MinFloatNode {}

impl MinFloatNode {
    pub fn new() -> Self {
        MinFloatNode {}
    }
}

#[async_trait]
impl NodeLogic for MinFloatNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "float_min",
            "Min",
            "Returns the smaller of two floats",
            "Math/Float",
        );
        node.add_icon("/flow/icons/sigma.svg");

        node.add_input_pin("float1", "Float 1", "First Float", VariableType::Float);
        node.add_input_pin("float2", "Float 2", "Second Float", VariableType::Float);

        node.add_output_pin(
            "minimum",
            "Minimum",
            "The smaller of the two floats",
            VariableType::Float,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) ->flow_like_types::Result<()> {
        let float1: f64 = context.evaluate_pin("float1").await?;
        let float2: f64 = context.evaluate_pin("float2").await?;

        let minimum = float1.min(float2);

        context.set_pin_value("minimum", json!(minimum)).await?;
        Ok(())
    }
}
