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
pub struct EvalNode {}

impl EvalNode {
    pub fn new() -> Self {
        EvalNode {}
    }
}

#[async_trait]
impl NodeLogic for EvalNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "eval",
            "Evaluate Expression",
            "Evaluates a mathematical expression",
            "Math",
        );
        node.add_icon("/flow/icons/calculator.svg");

        node.add_input_pin(
            "expression",
            "Expression",
            "Mathematical expression",
            VariableType::String,
        );

        node.add_output_pin(
            "result",
            "Result",
            "Result of the expression",
            VariableType::Float,
        );

        return node;
    }

    async fn run(&mut self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        let expression: String = context.evaluate_pin("expression").await?;
        let mut ns = fasteval::EmptyNamespace;
        let result = match fasteval::ez_eval(expression.as_str(), &mut ns) {
            Ok(result) => result,
            Err(e) => {
                let error: &str = &format!("Error evaluating expression: {}", e);
                context.log_message(error, crate::flow::execution::LogLevel::Error);
                0.0 // Or another appropriate default value
            }
        };

        context.set_pin_value("result", json!(result)).await?;
        Ok(())
    }
}
