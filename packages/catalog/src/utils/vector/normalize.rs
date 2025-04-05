use std::{collections::{HashMap, HashSet}, sync::Arc};
use flow_like::{flow::{board::Board, execution::{context::ExecutionContext, internal_node::InternalNode, log::LogMessage, LogLevel}, node::{Node, NodeLogic}, pin::{PinOptions, PinType, ValueType}, variable::{Variable, VariableType}}, state::FlowLikeState};
use flow_like_types::{async_trait, json::json, reqwest, sync::{DashMap, Mutex}, Value};
use nalgebra::DVector;
use regex::Regex;

use crate::{storage::path::FlowPath, web::api::{HttpBody, HttpRequest, HttpResponse, Method}};

#[derive(Default)]
pub struct FloatVectorNormalizeNode {}

impl FloatVectorNormalizeNode {
    pub fn new() -> Self {
        FloatVectorNormalizeNode {}
    }
}

#[async_trait]
impl NodeLogic for FloatVectorNormalizeNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "float_vector_normalize",
            "Normalize",
            "Normalizes a float vector",
            "Utils/Math/Vector",
        );
        node.add_icon("/flow/icons/grip.svg");

        node.add_input_pin(
            "vector",
            "Vector",
            "Float vector to normalize",
            VariableType::Float,
        )
        .set_value_type(ValueType::Array);

        node.add_output_pin(
            "normalized_vector",
            "Normalized Vector",
            "Normalized float vector",
            VariableType::Float,
        )
        .set_value_type(ValueType::Array);

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) ->flow_like_types::Result<()> {
        let vector: Vec<f64> = context.evaluate_pin("vector").await?;

        let v = DVector::from_vec(vector);

        let normalized_vector = v.normalize();

        context
            .set_pin_value(
                "normalized_vector",
                json!(normalized_vector.iter().cloned().collect::<Vec<_>>()),
            )
            .await?;
        Ok(())
    }
}
