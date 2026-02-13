use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic, NodeScores},
    variable::VariableType,
};
use flow_like_types::{async_trait, json::json};

/// Check if the current execution is by a technical user (API key).
#[crate::register_node]
#[derive(Default)]
pub struct IsTechnicalUserNode {}

impl IsTechnicalUserNode {
    pub fn new() -> Self {
        IsTechnicalUserNode {}
    }
}

#[async_trait]
impl NodeLogic for IsTechnicalUserNode {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "utils_user_is_technical_user",
            "Is Technical User",
            "Checks if the current execution is triggered by a technical user (API key) rather than a human user. Technical users don't have a human identity (sub) but do have a key_id.",
            "Utils/User",
        );
        node.add_icon("/flow/icons/key.svg");

        node.add_output_pin(
            "is_technical",
            "Is Technical User",
            "True if the execution is by a technical user (API key), false otherwise",
            VariableType::Boolean,
        );

        node.add_output_pin(
            "key_id",
            "Key ID",
            "The API key identifier for technical users, empty string for human users",
            VariableType::String,
        );

        node.set_scores(
            NodeScores::new()
                .set_privacy(5) // Returns minimal user info
                .set_security(9) // Read-only, useful for security checks
                .set_performance(10) // Very fast
                .set_governance(9) // Good for audit trails
                .set_reliability(10) // Always succeeds
                .set_cost(10) // No external calls
                .build(),
        );

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let user_context = context.user_context().cloned();

        match user_context {
            Some(uc) => {
                context
                    .set_pin_value("is_technical", json!(uc.is_technical()))
                    .await?;
                context
                    .set_pin_value("key_id", json!(uc.get_key_id().unwrap_or("")))
                    .await?;
            }
            None => {
                context.set_pin_value("is_technical", json!(false)).await?;
                context.set_pin_value("key_id", json!("")).await?;
            }
        }

        Ok(())
    }
}
