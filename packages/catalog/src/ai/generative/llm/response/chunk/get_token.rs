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
pub struct GetTokenNode {}

impl GetTokenNode {
    pub fn new() -> Self {
        GetTokenNode {}
    }
}

#[async_trait]
impl NodeLogic for GetTokenNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "ai_generative_llm_response_chunk_get_token",
            "Get Token",
            "Extracts the token from a ResponseChunk",
            "AI/Generative/Response/Chunk",
        );
        node.add_icon("/flow/icons/history.svg");

        node.add_input_pin(
            "chunk",
            "Chunk",
            "Response chunk to extract from",
            VariableType::Struct,
        )
        .set_schema::<ResponseChunk>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_output_pin(
            "token",
            "Token",
            "Token extracted from the response chunk",
            VariableType::String,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) ->flow_like_types::Result<()> {
        let chunk: ResponseChunk = context.evaluate_pin("chunk").await?;

        let token = chunk.get_streamed_token().unwrap_or_default();
        context.set_pin_value("token", json!(token)).await?;

        Ok(())
    }
}
