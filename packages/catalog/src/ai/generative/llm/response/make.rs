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
pub struct MakeResponseNode {}

impl MakeResponseNode {
    pub fn new() -> Self {
        MakeResponseNode {}
    }
}

#[async_trait]
impl NodeLogic for MakeResponseNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "ai_generative_llm_response_make",
            "Make Response",
            "",
            "AI/Generative/Response",
        );
        node.add_icon("/flow/icons/history.svg");

        node.add_output_pin("response", "Response", "", VariableType::Struct)
            .set_schema::<Response>()
            .set_options(PinOptions::new().set_enforce_schema(true).build());

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) ->flow_like_types::Result<()> {
        let response = Response::new();

        context.set_pin_value("response", json!(response)).await?;

        Ok(())
    }
}
