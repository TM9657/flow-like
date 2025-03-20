use crate::{
    flow::{
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        pin::PinOptions,
        variable::VariableType,
    },
    state::FlowLikeState,
};
use async_trait::async_trait;
use rand::Rng;
use serde_json::json;

#[derive(Default)]
pub struct RandomBoolNode {}

impl RandomBoolNode {
    pub fn new() -> Self {
        RandomBoolNode {}
    }
}

#[async_trait]
impl NodeLogic for RandomBoolNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "random_bool",
            "Random Boolean",
            "Generates a random boolean value",
            "Utils/Bool",
        );
        node.add_icon("/flow/icons/random.svg");

        node.add_input_pin(
            "probability",
            "Probability",
            "The probability of the boolean being true",
            VariableType::Float,
        )
        .set_default_value(Some(json!(0.5)))
        .set_options(PinOptions::new().set_range((0.0, 1.0)).build());

        node.add_output_pin(
            "value",
            "Value",
            "The random boolean value",
            VariableType::Boolean,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        let probability: f64 = context.evaluate_pin("probability").await?;
        let random_bool = {
            let mut rng = rand::rng();
            rng.random_bool(probability)
        };

        context.set_pin_value("value", json!(random_bool)).await?;
        Ok(())
    }
}
