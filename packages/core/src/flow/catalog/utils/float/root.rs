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
pub struct RootFloatNode {}

impl RootFloatNode {
    pub fn new() -> Self {
        RootFloatNode {}
    }
}

#[async_trait]
impl NodeLogic for RootFloatNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "float_root",
            "Root",
            "Calculates the nth root of a float",
            "Math/Float",
        );
        node.add_icon("/flow/icons/sigma.svg");

        node.add_input_pin(
            "radicand",
            "Radicand",
            "The float to take the root of",
            VariableType::Float,
        );
        node.add_input_pin(
            "degree",
            "Degree",
            "The degree of the root",
            VariableType::Integer,
        );

        node.add_output_pin(
            "root",
            "Root",
            "Result of the root calculation",
            VariableType::Float,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        let radicand: f64 = context.evaluate_pin("radicand").await?;
        let degree: i64 = context.evaluate_pin("degree").await?;

        if degree <= 0 {
            context.log_message(
                "Degree must be a positive integer",
                crate::flow::execution::LogLevel::Error,
            );
            context.set_pin_value("root", json!(0.0)).await?;
            return Ok(());
        }

        let root = radicand.powf(1.0 / degree as f64);
        context.set_pin_value("root", json!(root)).await?;

        Ok(())
    }
}
