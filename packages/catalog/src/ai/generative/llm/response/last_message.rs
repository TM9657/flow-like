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
pub struct LastMessageNode {}

impl LastMessageNode {
    pub fn new() -> Self {
        LastMessageNode {}
    }
}

#[async_trait]
impl NodeLogic for LastMessageNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "ai_generative_llm_response_last_message",
            "Last Message",
            "Extracts the last message from a Response",
            "AI/Generative/Response",
        );
        node.add_icon("/flow/icons/history.svg");

        node.add_input_pin(
            "response",
            "Response",
            "Response to extract from",
            VariableType::Struct,
        )
        .set_schema::<Response>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_output_pin(
            "message",
            "Message",
            "Last message from the response",
            VariableType::Struct,
        )
        .set_schema::<ResponseMessage>();

        node.add_output_pin(
            "success",
            "Success",
            "Whether a message was successfully extracted",
            VariableType::Boolean,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) ->flow_like_types::Result<()> {
        let response: Response = context.evaluate_pin("response").await?;

        if let Some(message) = response.last_message() {
            context.set_pin_value("message", json!(message)).await?;
            context.set_pin_value("success", json!(true)).await?;
        } else {
            context.set_pin_value("success", json!(false)).await?;
        }

        Ok(())
    }
}
