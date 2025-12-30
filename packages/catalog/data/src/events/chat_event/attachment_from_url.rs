use flow_like::{
    flow::{
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_types::{async_trait, json::json};

use super::Attachment;

#[crate::register_node]
#[derive(Default)]
pub struct AttachmentFromUrlNode {}

impl AttachmentFromUrlNode {
    pub fn new() -> Self {
        AttachmentFromUrlNode {}
    }
}

#[async_trait]
impl NodeLogic for AttachmentFromUrlNode {
    fn get_node(&self) -> Node {
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

        node.add_input_pin("signed_url", "Signed URL", "", VariableType::String);

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let signed_url: String = context.evaluate_pin("signed_url").await?;
        let attachment = Attachment::Url(signed_url.clone());

        context
            .set_pin_value("attachment", json!(attachment))
            .await?;

        return Ok(());
    }
}
