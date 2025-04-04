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
use flow_like_model_provider::response::ResponseMessage;
use serde_json::json;
#[derive(Default)]
pub struct GetContentNode {}

impl GetContentNode {
    pub fn new() -> Self {
        GetContentNode {}
    }
}

#[async_trait]
impl NodeLogic for GetContentNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "ai_generative_llm_response_message_get_content",
            "Get Content",
            "Extracts the content from a message",
            "AI/Generative/Response/Message",
        );
        node.add_icon("/flow/icons/history.svg");

        node.add_input_pin(
            "message",
            "Message",
            "Message to extract content from",
            VariableType::Struct,
        )
        .set_schema::<ResponseMessage>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_output_pin(
            "content",
            "Content",
            "Content string from the message",
            VariableType::String,
        );

        node.add_output_pin(
            "success",
            "Success",
            "Whether content was successfully extracted",
            VariableType::Boolean,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        let message: ResponseMessage = context.evaluate_pin("message").await?;

        if let Some(content) = message.content.as_ref() {
            context.set_pin_value("content", json!(content)).await?;
            context.set_pin_value("success", json!(true)).await?;
        } else {
            context.set_pin_value("content", json!("")).await?;
            context.set_pin_value("success", json!(false)).await?;
        }

        Ok(())
    }
}
