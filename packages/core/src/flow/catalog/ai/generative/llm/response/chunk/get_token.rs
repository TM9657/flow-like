use crate::{
    flow::{
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        pin::PinOptions,
        variable::VariableType,
    },
    models::response_chunk::ResponseChunk,
    state::FlowLikeState,
};
use async_trait::async_trait;
use serde_json::json;

#[derive(Default)]
pub struct GetTokenNode {}

impl GetTokenNode {
    pub fn new() -> Self {
        GetTokenNode {}
    }
}

#[async_trait]
impl NodeLogic for GetTokenNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "ai_generative_llm_response_chunk_get_token",
            "Get Token",
            "Extracts the token from a ResponseChunk",
            "AI/Generative/Response/Chunk",
        );
        node.add_icon("/flow/icons/history.svg");

        node.add_input_pin("chunk", "Chunk", "Response chunk to extract from", VariableType::Struct)
            .set_schema::<ResponseChunk>()
            .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_output_pin(
            "token",
            "Token",
            "Token extracted from the response chunk",
            VariableType::String,
        );

        return node;
    }

    async fn run(&mut self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        let chunk: ResponseChunk = context.evaluate_pin("chunk").await?;

        let token = chunk.get_streamed_token().unwrap_or_default();
        context.set_pin_value("token", json!(token)).await?;

        Ok(())
    }
}
