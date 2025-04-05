use std::{collections::{HashMap, HashSet}, sync::Arc};
use flow_like::{flow::{board::Board, execution::{context::ExecutionContext, internal_node::InternalNode, log::LogMessage, LogLevel}, node::{Node, NodeLogic}, pin::{PinOptions, ValueType}, variable::{Variable, VariableType}}, state::FlowLikeState};
use flow_like_types::{async_trait, json::json, reqwest, sync::{DashMap, Mutex}, Value};

use crate::{storage::path::FlowPath, web::api::{HttpBody, HttpRequest, HttpResponse, Method}};

#[derive(Default)]
pub struct PowerFloatNode {}

impl PowerFloatNode {
    pub fn new() -> Self {
        PowerFloatNode {}
    }
}

#[async_trait]
impl NodeLogic for PowerFloatNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "float_power",
            "Power",
            "Calculates the power of a float",
            "Math/Float",
        );
        node.add_icon("/flow/icons/sigma.svg");

        node.add_input_pin("base", "Base", "Base float", VariableType::Float);
        node.add_input_pin(
            "exponent",
            "Exponent",
            "Exponent float",
            VariableType::Float,
        );

        node.add_output_pin(
            "power",
            "Power",
            "Result of the power calculation",
            VariableType::Float,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) ->flow_like_types::Result<()> {
        let base: f64 = context.evaluate_pin("base").await?;
        let exponent: f64 = context.evaluate_pin("exponent").await?;

        let power = base.powf(exponent);
        context.set_pin_value("power", json!(power)).await?;

        Ok(())
    }
}
