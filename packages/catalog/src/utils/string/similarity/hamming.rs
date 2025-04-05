use std::{collections::{HashMap, HashSet}, sync::Arc};
use flow_like::{flow::{board::Board, execution::{context::ExecutionContext, internal_node::InternalNode, log::LogMessage, LogLevel}, node::{Node, NodeLogic}, pin::{PinOptions, ValueType}, variable::{Variable, VariableType}}, state::FlowLikeState};
use flow_like_types::{async_trait, json::json, reqwest, sync::{DashMap, Mutex}, Value};
use strsim::hamming;

use crate::{storage::path::FlowPath, web::api::{HttpBody, HttpRequest, HttpResponse, Method}};

#[derive(Default)]
pub struct HammingDistanceNode {}

impl HammingDistanceNode {
    pub fn new() -> Self {
        HammingDistanceNode {}
    }
}

#[async_trait]
impl NodeLogic for HammingDistanceNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "hamming_distance",
            "Hamming Distance",
            "Calculates the Hamming distance between two strings",
            "Utils/String/Similarity",
        );
        node.add_icon("/flow/icons/distance.svg");

        node.add_input_pin("string1", "String 1", "First String", VariableType::String);
        node.add_input_pin("string2", "String 2", "Second String", VariableType::String);

        node.add_output_pin(
            "distance",
            "Distance",
            "Hamming Distance",
            VariableType::Float,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) ->flow_like_types::Result<()> {
        let string1: String = context.evaluate_pin("string1").await?;
        let string2: String = context.evaluate_pin("string2").await?;

        let distance = hamming(&string1, &string2);

        let distance = match distance {
            Ok(distance) => distance,
            Err(e) => {
                context.log_message(
                    &format!("Error calculating Hamming distance: {:?}", e),
                    LogLevel::Error,
                );
                0
            }
        };

        context
            .set_pin_value("distance", json!(distance as f64))
            .await?;

        Ok(())
    }
}
