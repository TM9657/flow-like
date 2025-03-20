use crate::{
    flow::{
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        variable::VariableType,
    },
    state::FlowLikeState,
};
use async_trait::async_trait;
use rand::Rng;
use serde_json::json;

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

    async fn run(&self, context: &mut ExecutionContext) -> anyhow::Result<()> {
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
