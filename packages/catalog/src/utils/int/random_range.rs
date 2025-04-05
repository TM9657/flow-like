use std::{collections::{HashMap, HashSet}, sync::Arc};
use flow_like::{flow::{board::Board, execution::{context::ExecutionContext, internal_node::InternalNode, log::LogMessage, LogLevel}, node::{Node, NodeLogic}, pin::{PinOptions, ValueType}, variable::{Variable, VariableType}}, state::FlowLikeState};
use flow_like_types::{async_trait, json::json, rand::{self, Rng}, reqwest, sync::{DashMap, Mutex}, Value};

use crate::{storage::path::FlowPath, web::api::{HttpBody, HttpRequest, HttpResponse, Method}};

#[derive(Default)]
pub struct RandomIntegerInRangeNode {}

impl RandomIntegerInRangeNode {
    pub fn new() -> Self {
        RandomIntegerInRangeNode {}
    }
}

#[async_trait]
impl NodeLogic for RandomIntegerInRangeNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "int_random_in_range",
            "Random Integer in Range",
            "Generates a random integer within a specified range",
            "Math/Int/Random",
        );
        node.add_icon("/flow/icons/random.svg");

        node.add_input_pin("min", "Min", "Minimum Value", VariableType::Integer);
        node.add_input_pin("max", "Max", "Maximum Value", VariableType::Integer);

        node.add_output_pin(
            "random_integer",
            "Random Integer",
            "The generated random integer",
            VariableType::Integer,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) ->flow_like_types::Result<()> {
        let min: i64 = context.evaluate_pin("min").await?;
        let max: i64 = context.evaluate_pin("max").await?;

        let random_integer = {
            let mut rng = rand::rng();
            rng.random_range(min..=max) // Inclusive range for integers
        };

        context
            .set_pin_value("random_integer", json!(random_integer))
            .await?;
        Ok(())
    }
}
