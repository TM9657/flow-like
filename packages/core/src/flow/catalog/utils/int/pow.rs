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
pub struct PowerIntegerNode {}

impl PowerIntegerNode {
    pub fn new() -> Self {
        PowerIntegerNode {}
    }
}

#[async_trait]
impl NodeLogic for PowerIntegerNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "int_power",
            "Power",
            "Calculates the power of an integer",
            "Math/Int",
        );
        node.add_icon("/flow/icons/sigma.svg");

        node.add_input_pin("base", "Base", "Base integer", VariableType::Integer);
        node.add_input_pin(
            "exponent",
            "Exponent",
            "Exponent integer",
            VariableType::Integer,
        );

        node.add_output_pin(
            "power",
            "Power",
            "Result of the power calculation",
            VariableType::Integer,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        let base: i64 = context.evaluate_pin("base").await?;
        let exponent: i64 = context.evaluate_pin("exponent").await?;

        let power = base.pow(exponent as u32);
        context.set_pin_value("power", json!(power)).await?;

        Ok(())
    }
}
