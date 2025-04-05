use std::{collections::{HashMap, HashSet}, sync::Arc};
use flow_like::{flow::{board::Board, execution::{context::ExecutionContext, internal_node::InternalNode, log::LogMessage, LogLevel}, node::{Node, NodeLogic}, pin::{PinOptions, ValueType}, variable::{Variable, VariableType}}, state::FlowLikeState};
use flow_like_types::{async_trait, json::json, reqwest, sync::{DashMap, Mutex}, Value};

use crate::{storage::path::FlowPath, web::api::{HttpBody, HttpRequest, HttpResponse, Method}};

#[derive(Default)]
pub struct GreaterThanOrEqualFloatNode {}

impl GreaterThanOrEqualFloatNode {
    pub fn new() -> Self {
        GreaterThanOrEqualFloatNode {}
    }
}

#[async_trait]
impl NodeLogic for GreaterThanOrEqualFloatNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "float_greater_than_or_equal",
            ">=",
            "Checks if one float is greater than or equal to another",
            "Math/Float/Comparison",
        );
        node.add_icon("/flow/icons/sigma.svg");

        node.add_input_pin("float1", "Float 1", "First Float", VariableType::Float);
        node.add_input_pin("float2", "Float 2", "Second Float", VariableType::Float);

        node.add_output_pin(
            "is_greater_or_equal",
            "Is Greater or Equal",
            "True if float1 is greater than or equal to float2, false otherwise",
            VariableType::Boolean,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) ->flow_like_types::Result<()> {
        let float1: f64 = context.evaluate_pin("float1").await?;
        let float2: f64 = context.evaluate_pin("float2").await?;

        let is_greater_or_equal = float1 >= float2;

        context
            .set_pin_value("is_greater_or_equal", json!(is_greater_or_equal))
            .await?;
        Ok(())
    }
}
