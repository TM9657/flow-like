use std::{collections::{HashMap, HashSet}, sync::{atomic::{AtomicUsize, Ordering}, Arc}, time::Duration};
use flow_like::{bit::{Bit, BitTypes}, flow::{board::Board, execution::{context::ExecutionContext, internal_node::InternalNode, log::{LogMessage, LogStat}, LogLevel}, node::{Node, NodeLogic}, pin::{PinOptions, PinType, ValueType}, variable::{Variable, VariableType}}, state::{FlowLikeState, ToastLevel}};
use flow_like_model_provider::{history::{History, HistoryMessage, Role}, llm::LLMCallback, response::{Response, ResponseMessage}, response_chunk::ResponseChunk};
use flow_like_types::{Result, async_trait, json::{from_str, json, Deserialize, Serialize}, reqwest, sync::{DashMap, Mutex}, Bytes, Error, JsonSchema, Value};
use nalgebra::DVector;
use regex::Regex;
use flow_like_storage::{object_store::PutPayload, Path};
use futures::StreamExt;
use crate::{storage::path::FlowPath, web::api::{HttpBody, HttpRequest, HttpResponse, Method}};

#[derive(Default)]
pub struct GetRoleNode {}

impl GetRoleNode {
    pub fn new() -> Self {
        GetRoleNode {}
    }
}

#[async_trait]
impl NodeLogic for GetRoleNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "ai_generative_llm_response_message_get_role",
            "Get Role",
            "Extracts the role from a message",
            "AI/Generative/Response/Message",
        );
        node.add_icon("/flow/icons/history.svg");

        node.add_input_pin(
            "message",
            "Message",
            "Message to extract role from",
            VariableType::Struct,
        )
        .set_schema::<ResponseMessage>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_output_pin(
            "role",
            "Role",
            "Role string from the message",
            VariableType::String,
        )
        .set_options(
            PinOptions::new()
                .set_valid_values(vec![
                    "system".to_string(),
                    "user".to_string(),
                    "assistant".to_string(),
                ])
                .build(),
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) ->flow_like_types::Result<()> {
        let message: ResponseMessage = context.evaluate_pin("message").await?;

        context.set_pin_value("role", json!(message.role)).await?;
        Ok(())
    }
}
