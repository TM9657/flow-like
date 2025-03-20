use strsim::jaro;

use crate::{
    flow::{
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        variable::VariableType,
    },
    state::FlowLikeState,
};
use async_trait::async_trait;
use serde_json::json;

#[derive(Default)]
pub struct JaroDistanceNode {}

impl JaroDistanceNode {
    pub fn new() -> Self {
        JaroDistanceNode {}
    }
}

#[async_trait]
impl NodeLogic for JaroDistanceNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "jaro_distance",
            "Jaro Distance",
            "Calculates the Jaro distance between two strings",
            "Utils/String/Similarity",
        );
        node.add_icon("/flow/icons/distance.svg");

        node.add_input_pin("string1", "String 1", "First String", VariableType::String);
        node.add_input_pin("string2", "String 2", "Second String", VariableType::String);

        node.add_output_pin("distance", "Distance", "Jaro Distance", VariableType::Float);

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        let string1: String = context.evaluate_pin("string1").await?;
        let string2: String = context.evaluate_pin("string2").await?;

        let distance = jaro(&string1, &string2);

        context.set_pin_value("distance", json!(distance)).await?;

        Ok(())
    }
}
