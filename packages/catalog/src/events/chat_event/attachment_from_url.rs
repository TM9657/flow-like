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
pub struct AttachmentFromUrlNode {}

impl AttachmentFromUrlNode {
    pub fn new() -> Self {
        AttachmentFromUrlNode {}
    }
}

#[async_trait]
impl NodeLogic for AttachmentFromUrlNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "events_chat_attachment_from_signed_url",
            "From Signed URL",
            "Get the URL from an attachment",
            "Events/Chat/Attachments",
        );
        node.add_icon("/flow/icons/paperclip.svg");

        node.add_output_pin(
            "attachment",
            "Attachment",
            "Attachment to the Chat",
            VariableType::Struct,
        )
        .set_schema::<Attachment>();

        node.add_input_pin(
            "signed_url",
            "Signed URL",
            "",
            VariableType::String,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let signed_url: String = context.evaluate_pin("signed_url").await?;
        let attachment = Attachment::Url(signed_url.clone());

        context.set_pin_value("attachment", json!(attachment)).await?;

        return Ok(());
    }
}
