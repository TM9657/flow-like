use std::{collections::{HashMap, HashSet}, sync::{atomic::{AtomicUsize, Ordering}, Arc}, time::Duration};
use flow_like::{bit::{Bit, BitTypes}, flow::{board::Board, execution::{context::ExecutionContext, internal_node::InternalNode, log::{LogMessage, LogStat}, LogLevel}, node::{Node, NodeLogic}, pin::{PinOptions, PinType, ValueType}, variable::{Variable, VariableType}}, state::{FlowLikeState, ToastLevel}};
use flow_like_model_provider::{history::{History, HistoryMessage, Role}, llm::LLMCallback, response::Response, response_chunk::ResponseChunk};
use flow_like_types::{Result, async_trait, json::{from_str, json, Deserialize, Serialize}, reqwest, sync::{DashMap, Mutex}, Bytes, Error, JsonSchema, Value};
use nalgebra::DVector;
use regex::Regex;
use flow_like_storage::{object_store::PutPayload, Path};
use futures::StreamExt;
use crate::{storage::path::FlowPath, web::api::{HttpBody, HttpRequest, HttpResponse, Method}};

#[derive(Default)]
pub struct PushChunkNode {}

impl PushChunkNode {
    pub fn new() -> Self {
        PushChunkNode {}
    }
}

#[async_trait]
impl NodeLogic for PushChunkNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "ai_generative_llm_response_push_chunk",
            "Push Chunk",
            "Adds a response chunk to a Response",
            "AI/Generative/Response",
        );
        node.add_icon("/flow/icons/history.svg");

        node.add_input_pin(
            "exec_in",
            "Input",
            "Initiate Execution",
            VariableType::Execution,
        );

        node.add_input_pin(
            "response",
            "Response",
            "Response to update",
            VariableType::Struct,
        )
        .set_schema::<Response>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_input_pin(
            "chunk",
            "Chunk",
            "Response chunk to add",
            VariableType::Struct,
        )
        .set_schema::<ResponseChunk>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_output_pin(
            "exec_out",
            "Output",
            "Done with the Execution",
            VariableType::Execution,
        );

        node.add_output_pin(
            "response_out",
            "Response",
            "Updated Response",
            VariableType::Struct,
        )
        .set_schema::<Response>();

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) ->flow_like_types::Result<()> {
        let mut response: Response = context.evaluate_pin("response").await?;
        let chunk: ResponseChunk = context.evaluate_pin("chunk").await?;

        response.push_chunk(chunk);

        context
            .set_pin_value("response_out", json!(response))
            .await?;
        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }
}
