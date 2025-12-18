use flow_like::{
    flow::{
        board::Board,
        execution::context::ExecutionContext,
        node::{Node, NodeLogic, NodeScores},
        pin::PinOptions,
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_model_provider::history::{
    Content, ContentType, HistoryMessage, ImageUrl, MessageContent,
};
use flow_like_types::{Value, async_trait, json::json};
use std::sync::Arc;

#[crate::register_node]
#[derive(Default)]
pub struct PushContentNode {}

impl PushContentNode {
    pub fn new() -> Self {
        PushContentNode {}
    }

    fn add_type_pin(node: &mut Node) {
        node.add_input_pin("type", "Type", "Content type", VariableType::String)
            .set_options(
                PinOptions::new()
                    .set_valid_values(vec!["Text".to_string(), "Image".to_string()])
                    .build(),
            )
            .set_default_value(Some(json!("Text")));
    }

    fn normalized_content(message: &HistoryMessage) -> Vec<Content> {
        match &message.content {
            MessageContent::String(text) => vec![Content::Text {
                content_type: ContentType::Text,
                text: text.clone(),
            }],
            MessageContent::Contents(contents) => contents.clone(),
        }
    }
}

#[async_trait]
impl NodeLogic for PushContentNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "ai_generative_push_content",
            "Push Content",
            "Appends text or image parts onto a chat message",
            "AI/Generative/History/Message",
        );
        node.add_icon("/flow/icons/message.svg");
        node.set_scores(
            NodeScores::new()
                .set_privacy(10)
                .set_security(10)
                .set_performance(9)
                .set_reliability(10)
                .set_governance(9)
                .set_cost(10)
                .build(),
        );

        node.add_input_pin(
            "exec_in",
            "Input",
            "Trigger when ready to append content",
            VariableType::Execution,
        );

        node.add_input_pin(
            "message",
            "Message",
            "Message to extend",
            VariableType::Struct,
        )
        .set_schema::<HistoryMessage>();

        Self::add_type_pin(&mut node);

        node.add_output_pin(
            "exec_out",
            "Output",
            "Signals completion once content is appended",
            VariableType::Execution,
        );

        node.add_output_pin(
            "message_out",
            "Message",
            "Updated message with additional content",
            VariableType::Struct,
        )
        .set_schema::<HistoryMessage>();

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;

        let mut message: HistoryMessage = context.evaluate_pin("message").await?;
        let content_type: String = context.evaluate_pin("type").await?;
        let mut content = Self::normalized_content(&message);

        match content_type.as_str() {
            "Text" => {
                let text: String = context.evaluate_pin("text").await?;
                content.push(Content::Text {
                    content_type: ContentType::Text,
                    text,
                });
            }
            "Image" => {
                let image: String = context.evaluate_pin("image").await?;
                let mime: String = context.evaluate_pin("mime").await?;
                content.push(Content::Image {
                    content_type: ContentType::ImageUrl,
                    image_url: ImageUrl {
                        url: image,
                        detail: Some(mime),
                    },
                });
            }
            _ => {}
        }

        message.content = MessageContent::Contents(content);

        context.set_pin_value("message_out", json!(message)).await?;
        context.activate_exec_pin("exec_out").await?;
        Ok(())
    }

    async fn on_update(&self, node: &mut Node, _board: Arc<Board>) {
        let type_pin: String = node
            .get_pin_by_name("type")
            .and_then(|pin| pin.default_value.clone())
            .and_then(|bytes| flow_like_types::json::from_slice::<Value>(&bytes).ok())
            .and_then(|json| json.as_str().map(ToOwned::to_owned))
            .unwrap_or_default();

        let text_pin = node.get_pin_by_name("text");
        let image_pin = node.get_pin_by_name("image");
        let mime_pin = node.get_pin_by_name("mime");

        if type_pin == *"Text" && text_pin.is_some() {
            return;
        }

        if type_pin == *"Image" && image_pin.is_some() && mime_pin.is_some() {
            return;
        }

        let mut removal = vec![];

        if type_pin == "Text" {
            if let Some(image_pin) = image_pin {
                removal.push(image_pin.id.clone());
            }

            if let Some(mime_pin) = mime_pin {
                removal.push(mime_pin.id.clone());
            }

            for id in removal {
                node.pins.remove(&id);
            }

            node.add_input_pin("text", "Text", "Text Content", VariableType::String);
            return;
        }

        if let Some(text_pin) = text_pin {
            removal.push(text_pin.id.clone());
        }

        for id in removal {
            node.pins.remove(&id);
        }

        node.add_input_pin("image", "Image", "Image Content", VariableType::String);
        node.add_input_pin("mime", "Mime", "Mime Type", VariableType::String);
    }
}
