use crate::{
    flow::{
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        pin::PinOptions,
        variable::VariableType,
    },
    models::response::ResponseMessage,
    state::FlowLikeState,
};
use async_trait::async_trait;
use serde_json::json;

#[derive(Default)]
pub struct GetRoleNode {}

impl GetRoleNode {
    pub fn new() -> Self {
        GetRoleNode {}
    }
}

#[async_trait]
impl NodeLogic for GetRoleNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "ai_generative_llm_response_message_get_role",
            "Get Role",
            "Extracts the role from a message",
            "AI/Generative/Response/Message",
        );
        node.add_icon("/flow/icons/history.svg");

        node.add_input_pin(
            "message",
            "Message",
            "Message to extract role from",
            VariableType::Struct,
        )
        .set_schema::<ResponseMessage>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_output_pin(
            "role",
            "Role",
            "Role string from the message",
            VariableType::String,
        )
        .set_options(
            PinOptions::new()
                .set_valid_values(vec![
                    "system".to_string(),
                    "user".to_string(),
                    "assistant".to_string(),
                ])
                .build(),
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        let message: ResponseMessage = context.evaluate_pin("message").await?;

        context.set_pin_value("role", json!(message.role)).await?;
        Ok(())
    }
}
