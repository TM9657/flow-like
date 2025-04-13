use std::sync::Arc;

use flow_like::{
    flow::{
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        pin::PinOptions,
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_model_provider::{
    history::{HistoryMessage, Role},
    response::Response,
    response_chunk::ResponseChunk,
};
use flow_like_types::{Value, async_trait, json::json, sync::Mutex};

use crate::events::chat_event::ChatResponse;

use super::{Attachment, CachedChatResponse, ChatStreamingResponse, Reasoning};

#[derive(Default)]
pub struct PushAttachmentNode {}

impl PushAttachmentNode {
    pub fn new() -> Self {
        PushAttachmentNode {}
    }
}

#[async_trait]
impl NodeLogic for PushAttachmentNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "events_chat_push_attachment",
            "Push Attachment",
            "Pushes a response chunk to the chat",
            "Events/Chat",
        );
        node.add_icon("/flow/icons/event.svg");
        node.set_event_callback(true);

        node.add_input_pin(
            "exec_in",
            "Input",
            "Initiate Execution",
            VariableType::Execution,
        );

        node.add_input_pin(
            "attachment",
            "Attachment",
            "Attachment to the Chat",
            VariableType::Struct,
        )
        .set_schema::<Attachment>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_output_pin(
            "exec_out",
            "Output",
            "Done with the Execution",
            VariableType::Execution,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let attachment: Attachment = context.evaluate_pin("attachment").await?;
        let cached_response = CachedChatResponse::load(context).await?;
        {
            let mut mutable_response = cached_response.response.lock().await;
            mutable_response.attachments.push(attachment.clone());
        }

        let streaming_response = ChatStreamingResponse {
            actions: vec![],
            attachments: vec![attachment],
            chunk: None,
            plan: None,
        };

        context
            .stream_response("chat_stream_partial", streaming_response)
            .await?;
        context.activate_exec_pin("exec_out").await?;

        return Ok(());
    }
}
