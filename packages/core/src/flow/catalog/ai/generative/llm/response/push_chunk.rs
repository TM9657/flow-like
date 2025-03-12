use crate::{
    flow::{
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        pin::PinOptions,
        variable::VariableType,
    },
    models::{response::Response, response_chunk::ResponseChunk},
    state::FlowLikeState,
};
use async_trait::async_trait;
use serde_json::json;

#[derive(Default)]
pub struct PushChunkNode {}

impl PushChunkNode {
    pub fn new() -> Self {
        PushChunkNode {}
    }
}

#[async_trait]
impl NodeLogic for PushChunkNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "ai_generative_llm_response_push_chunk",
            "Push Chunk",
            "Adds a response chunk to a Response",
            "AI/Generative/Response",
        );
        node.add_icon("/flow/icons/history.svg");

        node.add_input_pin(
            "exec_in",
            "Input",
            "Initiate Execution",
            VariableType::Execution,
        );

        node.add_input_pin(
            "response",
            "Response",
            "Response to update",
            VariableType::Struct,
        )
        .set_schema::<Response>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_input_pin(
            "chunk",
            "Chunk",
            "Response chunk to add",
            VariableType::Struct,
        )
        .set_schema::<ResponseChunk>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_output_pin(
            "exec_out",
            "Output",
            "Done with the Execution",
            VariableType::Execution,
        );

        node.add_output_pin(
            "response_out",
            "Response",
            "Updated Response",
            VariableType::Struct,
        )
        .set_schema::<Response>();

        return node;
    }

    async fn run(&mut self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        let mut response: Response = context.evaluate_pin("response").await?;
        let chunk: ResponseChunk = context.evaluate_pin("chunk").await?;

        response.push_chunk(chunk);

        context
            .set_pin_value("response_out", json!(response))
            .await?;
        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }
}
