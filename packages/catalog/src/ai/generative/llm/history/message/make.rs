use std::{collections::{HashMap, HashSet}, sync::{atomic::{AtomicUsize, Ordering}, Arc}, time::Duration};
use flow_like::{bit::{Bit, BitModelPreference, BitTypes}, flow::{board::Board, execution::{context::ExecutionContext, internal_node::InternalNode, log::{LogMessage, LogStat}, LogLevel}, node::{Node, NodeLogic}, pin::{PinOptions, PinType, ValueType}, variable::{Variable, VariableType}}, state::{FlowLikeState, ToastLevel}};
use flow_like_model_provider::{history::{Content, ContentType, History, HistoryMessage, Role}, llm::LLMCallback, response::{Response, ResponseMessage}, response_chunk::ResponseChunk};
use flow_like_types::{Result, async_trait, json::{from_str, json, Deserialize, Serialize}, reqwest, sync::{DashMap, Mutex}, Bytes, Error, JsonSchema, Value};
use nalgebra::DVector;
use regex::Regex;
use flow_like_storage::{object_store::PutPayload, Path};
use futures::StreamExt;
use crate::{storage::path::FlowPath, web::api::{HttpBody, HttpRequest, HttpResponse, Method}};

#[derive(Default)]
pub struct MakeHistoryMessageNode {}

impl MakeHistoryMessageNode {
    pub fn new() -> Self {
        MakeHistoryMessageNode {}
    }
}

#[async_trait]
impl NodeLogic for MakeHistoryMessageNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "ai_generative_make_history_message",
            "Make Message",
            "Creates a ChatHistory struct",
            "AI/Generative/History/Message",
        );
        node.add_icon("/flow/icons/message.svg");

        node.add_input_pin(
            "role",
            "Role",
            "The Role of the Message Author",
            VariableType::String,
        )
        .set_options(
            PinOptions::new()
                .set_valid_values(vec![
                    "Assistant".to_string(),
                    "System".to_string(),
                    "User".to_string(),
                ])
                .build(),
        )
        .set_default_value(Some(json!("User")));

        node.add_input_pin("type", "Type", "Message Type", VariableType::String)
            .set_options(
                PinOptions::new()
                    .set_valid_values(vec!["Text".to_string(), "Image".to_string()])
                    .build(),
            )
            .set_default_value(Some(json!("Text")));

        node.add_output_pin(
            "message",
            "Message",
            "Constructed Message",
            VariableType::Struct,
        )
        .set_schema::<HistoryMessage>();

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) ->flow_like_types::Result<()> {
        let role: String = context.evaluate_pin("role").await?;
        let message_type: String = context.evaluate_pin("type").await?;

        let role: Role = match role.as_str() {
            "Assistant" => Role::Assistant,
            "System" => Role::System,
            "User" => Role::User,
            _ => Role::User,
        };

        let mut message: HistoryMessage = HistoryMessage {
            content: vec![],
            role: Role::User,
        };

        message.role = role;

        match message_type.as_str() {
            "Text" => {
                let text_pin: String = context.evaluate_pin("text").await?;
                message.content = vec![Content::Text {
                    content_type: ContentType::Text,
                    text: text_pin,
                }];
            }
            "Image" => {
                let image_pin: String = context.evaluate_pin("image").await?;
                let mime_pin: String = context.evaluate_pin("mime").await?;
                message.content = vec![Content::Image {
                    content_type: ContentType::ImageUrl,
                    data: image_pin,
                    mime_type: mime_pin,
                }];
            }
            _ => {}
        }

        context.set_pin_value("message", json!(message)).await?;

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

        if (type_pin == *"Text") && text_pin.is_some() {
            return;
        }

        if (type_pin == *"Image") && image_pin.is_some() && mime_pin.is_some() {
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
