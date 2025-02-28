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
pub struct GetEnvVariableNode {}

impl GetEnvVariableNode {
    pub fn new() -> Self {
        GetEnvVariableNode {}
    }
}

#[async_trait]
impl NodeLogic for GetEnvVariableNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "get_env",
            "Get Environment Variable",
            "Get Environment Variable",
            "Utils/Env",
        );
        node.add_icon("/flow/icons/env.svg");

        node.add_input_pin("key", "Key", "Variable Key", VariableType::String);

        node.add_output_pin(
            "variable",
            "Variable",
            "Environment Variable",
            VariableType::String,
        );
        node.add_output_pin(
            "found",
            "Found?",
            "Is the environment variable set?",
            VariableType::Boolean,
        );

        return node;
    }

    async fn run(&mut self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        let key: String = context.evaluate_pin("key").await?;
        let variable = std::env::var(key);
        let found = variable.is_ok();
        let variable = variable.unwrap_or_default();
        context.set_pin_value("variable", json!(variable)).await?;
        context.set_pin_value("found", json!(found)).await?;
        Ok(())
    }
}
