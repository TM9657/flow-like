use std::sync::Arc;

use flow_like::{
    flow::{
        execution::{EventTrigger, context::ExecutionContext},
        node::{Node, NodeLogic},
        pin::PinOptions,
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_model_provider::{
    history::{History, HistoryMessage},
    response::Response,
    response_chunk::ResponseChunk,
};
use flow_like_types::{
    Cacheable, Value, anyhow, async_trait,
    intercom::InterComEvent,
    json::{from_str, json},
    sync::Mutex,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub mod attachment_from_url;
pub mod attachment_to_url;
pub mod push_attachment;
pub mod push_attachments;
pub mod push_chunk;
pub mod push_global_session;
pub mod push_local_session;
pub mod push_response;

#[derive(Default)]
pub struct ChatEventNode {}

impl ChatEventNode {
    pub fn new() -> Self {
        ChatEventNode {}
    }
}

#[async_trait]
impl NodeLogic for ChatEventNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new("events_chat", "Chat Event", "A simple Chat event", "Events");
        node.add_icon("/flow/icons/event.svg");
        node.set_start(true);

        node.add_output_pin(
            "exec_out",
            "Output",
            "Starting an event",
            VariableType::Execution,
        );

        node.add_output_pin("history", "History", "Chat History", VariableType::Struct)
            .set_schema::<History>()
            .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_output_pin(
            "local_session",
            "Local Session",
            "Local to the Chat",
            VariableType::Struct,
        );

        node.add_output_pin(
            "global_session",
            "Global Session",
            "Global to the User",
            VariableType::Struct,
        );

        node.add_output_pin("actions", "Actions", "User Actions", VariableType::Struct)
            .set_schema::<ChatAction>()
            .set_value_type(flow_like::flow::pin::ValueType::Array)
            .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_output_pin(
            "attachments",
            "Attachments",
            "User Attachments or References",
            VariableType::Struct,
        )
        .set_schema::<Attachment>()
        .set_value_type(flow_like::flow::pin::ValueType::Array)
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_output_pin("user", "User", "User Information", VariableType::Struct)
            .set_schema::<User>()
            .set_options(PinOptions::new().set_enforce_schema(true).build());

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let exec_out_pin = context.get_pin_by_name("exec_out").await?;
        let payload = context.get_payload().await?;
        let chat = payload.payload.ok_or(anyhow!("Failed to get payload"))?;
        let chat: Chat = flow_like_types::json::from_value(chat)
            .map_err(|e| anyhow!("Failed to deserialize payload: {}", e))?;

        context
            .set_pin_value(
                "history",
                json!(History::new("".to_string(), chat.messages)),
            )
            .await?;
        context
            .set_pin_value(
                "local_session",
                chat.local_session.unwrap_or(from_str("{}")?),
            )
            .await?;
        context
            .set_pin_value(
                "global_session",
                chat.global_session.unwrap_or(from_str("{}")?),
            )
            .await?;
        context
            .set_pin_value("actions", json!(chat.actions.unwrap_or_default()))
            .await?;
        context
            .set_pin_value("attachments", json!(chat.attachments.unwrap_or_default()))
            .await?;
        context
            .set_pin_value("user", json!(chat.user.unwrap_or_default()))
            .await?;
        context.activate_exec_pin_ref(&exec_out_pin).await?;

        let completion_event: EventTrigger = Arc::new(|run| {
            Box::pin(async move {
                if let Some(cached_response) = run.cache.read().await.get("chat_response") {
                    let cached_response = cached_response.clone();
                    let response = cached_response
                        .as_any()
                        .downcast_ref::<CachedChatResponse>()
                        .ok_or(anyhow!("Failed to downcast cached response"))?;

                    let event = {
                        let response = response.response.lock().await;
                        InterComEvent::with_type("chat_out", response.clone())
                    };
                    event.call(&run.callback).await?;
                }
                Ok(())
            })
        });

        context.hook_completion_event(completion_event).await;

        return Ok(());
    }
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
#[serde(untagged)]
pub enum Attachment {
    Url(String),
    Reference { id: String, resource_type: String },
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub enum ButtonType {
    Outline,
    Primary,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub enum ChatAction {
    Button(String, ButtonType),
    Form(String, Value),
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, Default)]
pub struct User {
    pub sub: String,
    pub name: String,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct Chat {
    pub chat_id: Option<String>,
    pub messages: Vec<HistoryMessage>,
    pub local_session: Option<Value>,
    pub global_session: Option<Value>,
    pub actions: Option<Vec<ChatAction>>,
    pub user: Option<User>,
    pub attachments: Option<Vec<Attachment>>,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct ChatResponse {
    pub response: Response,
    pub local_session: Option<Value>,
    pub global_session: Option<Value>,
    pub actions: Vec<ChatAction>,
    pub attachments: Vec<Attachment>,
    pub model_id: Option<String>,
}

#[derive(Clone)]
pub struct CachedChatResponse {
    response: Arc<Mutex<ChatResponse>>,
    reasoning: Arc<Mutex<Reasoning>>,
}

impl CachedChatResponse {
    pub async fn load(context: &mut ExecutionContext) -> flow_like_types::Result<Self> {
        if let Some(cached_response) = context.get_cache("chat_response").await {
            let response = cached_response
                .as_any()
                .downcast_ref::<CachedChatResponse>()
                .ok_or(anyhow!("Failed to downcast cached response"))?;
            return Ok(response.clone());
        }

        let response = ChatResponse {
            response: Response::new(),
            actions: vec![],
            attachments: vec![],
            global_session: flow_like_types::json::from_str("{}")?,
            local_session: flow_like_types::json::from_str("{}")?,
            model_id: None,
        };

        let reasoning = Reasoning {
            current_message: "".to_string(),
            current_step: 0,
            plan: vec![],
        };

        let cached_response = CachedChatResponse {
            response: Arc::new(Mutex::new(response)),
            reasoning: Arc::new(Mutex::new(reasoning)),
        };

        let cacheable = Arc::new(cached_response.clone()) as Arc<dyn Cacheable>;
        context.set_cache("chat_response", cacheable).await;
        Ok(cached_response)
    }
}

impl Cacheable for CachedChatResponse {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct Reasoning {
    pub plan: Vec<(u32, String)>,
    pub current_step: u32,
    pub current_message: String,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct ChatStreamingResponse {
    pub chunk: Option<ResponseChunk>,
    pub actions: Vec<ChatAction>,
    pub attachments: Vec<Attachment>,
    pub plan: Option<Reasoning>,
}

pub async fn register_functions() -> Vec<Arc<dyn NodeLogic>> {
    vec![
        Arc::new(ChatEventNode::default()) as Arc<dyn NodeLogic>,
        Arc::new(push_response::PushResponseNode::default()) as Arc<dyn NodeLogic>,
        Arc::new(push_chunk::PushChunkNode::default()) as Arc<dyn NodeLogic>,
        Arc::new(push_attachment::PushAttachmentNode::default()) as Arc<dyn NodeLogic>,
        Arc::new(push_attachments::PushAttachmentsNode::default()) as Arc<dyn NodeLogic>,
        Arc::new(attachment_to_url::AttachmentToUrlNode::default()) as Arc<dyn NodeLogic>,
        Arc::new(attachment_from_url::AttachmentFromUrlNode::default()) as Arc<dyn NodeLogic>,
        Arc::new(push_local_session::PushLocalSessionNode::default()) as Arc<dyn NodeLogic>,
        Arc::new(push_global_session::PushGlobalSessionNode::default()) as Arc<dyn NodeLogic>,
    ]
}
