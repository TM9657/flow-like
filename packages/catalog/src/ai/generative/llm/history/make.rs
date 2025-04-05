use std::{collections::{HashMap, HashSet}, sync::{atomic::{AtomicUsize, Ordering}, Arc}, time::Duration};
use flow_like::{bit::{Bit, BitModelPreference, BitTypes}, flow::{board::Board, execution::{context::ExecutionContext, internal_node::InternalNode, log::{LogMessage, LogStat}, LogLevel}, node::{Node, NodeLogic}, pin::{PinOptions, PinType, ValueType}, variable::{Variable, VariableType}}, state::{FlowLikeState, ToastLevel}};
use flow_like_model_provider::{history::{History, HistoryMessage, Role}, llm::LLMCallback, response::{Response, ResponseMessage}, response_chunk::ResponseChunk};
use flow_like_types::{Result, async_trait, json::{from_str, json, Deserialize, Serialize}, reqwest, sync::{DashMap, Mutex}, Bytes, Error, JsonSchema, Value};
use nalgebra::DVector;
use regex::Regex;
use flow_like_storage::{object_store::PutPayload, Path};
use futures::StreamExt;
use crate::{storage::path::FlowPath, web::api::{HttpBody, HttpRequest, HttpResponse, Method}};

#[derive(Default)]
pub struct MakeHistoryNode {}

impl MakeHistoryNode {
    pub fn new() -> Self {
        MakeHistoryNode {}
    }
}

#[async_trait]
impl NodeLogic for MakeHistoryNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "ai_generative_make_history",
            "Make History",
            "Creates a ChatHistory struct",
            "AI/Generative/History",
        );
        node.add_icon("/flow/icons/history.svg");

        node.add_input_pin(
            "model_name",
            "Model Name",
            "Model Name",
            VariableType::String,
        )
        .set_default_value(Some(json!("")));

        node.add_output_pin("history", "History", "ChatHistory", VariableType::Struct)
            .set_schema::<History>()
            .set_options(PinOptions::new().set_enforce_schema(true).build());

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) ->flow_like_types::Result<()> {
        let model_name: String = context.evaluate_pin("model_name").await?;
        let history = History::new(model_name, vec![]);

        context.set_pin_value("history", json!(history)).await?;

        Ok(())
    }
}
