use std::sync::Arc;

use crate::{
    flow::{
        board::Board,
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        pin::PinOptions,
        variable::VariableType,
    },
    models::history::{Content, HistoryMessage},
    state::FlowLikeState,
};
use async_trait::async_trait;
use serde_json::{json, Value};

#[derive(Default)]
pub struct PushContentNode {}

impl PushContentNode {
    pub fn new() -> Self {
        PushContentNode {}
    }
}

#[async_trait]
impl NodeLogic for PushContentNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "ai_generative_push_content",
            "Push Content",
            "Pushes content into a HistoryMessage",
            "AI/Generative/History/Message",
        );
        node.add_icon("/flow/icons/message.svg");

        node.add_input_pin(
            "exec_in",
            "Input",
            "Initiate Execution",
            VariableType::Execution,
        );

        node.add_input_pin("message", "Message", "Input Message", VariableType::Struct)
            .set_schema::<HistoryMessage>();

        node.add_input_pin("type", "Type", "Content Type", VariableType::String)
            .set_options(
                PinOptions::new()
                    .set_valid_values(vec!["Text".to_string(), "Image".to_string()])
                    .build(),
            )
            .set_default_value(Some(json!("Text")));

        node.add_output_pin(
            "exec_out",
            "Output",
            "Done with the Execution",
            VariableType::Execution,
        );

        node.add_output_pin(
            "message_out",
            "Message",
            "Output Message",
            VariableType::Struct,
        )
        .set_schema::<HistoryMessage>();

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        let mut message: HistoryMessage = context.evaluate_pin("message").await?;
        let content_type: String = context.evaluate_pin("type").await?;

        match content_type.as_str() {
            "Text" => {
                let text: String = context.evaluate_pin("text").await?;
                message.content.push(Content::Text {
                    content_type: crate::models::history::ContentType::Text,
                    text,
                });
            }
            "Image" => {
                let image: String = context.evaluate_pin("image").await?;
                let mime: String = context.evaluate_pin("mime").await?;
                message.content.push(Content::Image {
                    content_type: crate::models::history::ContentType::ImageUrl,
                    data: image,
                    mime_type: mime,
                });
            }
            _ => {}
        }

        context.set_pin_value("message_out", json!(message)).await?;
        context.activate_exec_pin("exec_out").await?;
        Ok(())
    }

    async fn on_update(&self, node: &mut Node, _board: Arc<Board>) {
        let type_pin: String = node
            .get_pin_by_name("type")
            .and_then(|pin| pin.default_value.clone())
            .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
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
