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
pub struct DivideFloatNode {}

impl DivideFloatNode {
    pub fn new() -> Self {
        DivideFloatNode {}
    }
}

#[async_trait]
impl NodeLogic for DivideFloatNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "float_divide",
            "/",
            "Divides one float by another",
            "Math/Float",
        );
        node.add_icon("/flow/icons/sigma.svg");

        node.add_input_pin(
            "dividend",
            "Dividend",
            "The number to be divided",
            VariableType::Float,
        );
        node.add_input_pin(
            "divisor",
            "Divisor",
            "The number to divide by",
            VariableType::Float,
        );

        node.add_output_pin(
            "quotient",
            "Quotient",
            "The result of the division",
            VariableType::Float,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        let dividend: f64 = context.evaluate_pin("dividend").await?;
        let divisor: f64 = context.evaluate_pin("divisor").await?;

        if divisor == 0.0 {
            context.log_message(
                "Division by zero error!",
                crate::flow::execution::LogLevel::Error,
            );
            context.set_pin_value("quotient", json!(0.0)).await?; // Or NaN, or handle differently
            return Ok(());
        }

        let quotient = dividend / divisor;

        context.set_pin_value("quotient", json!(quotient)).await?;
        Ok(())
    }
}
