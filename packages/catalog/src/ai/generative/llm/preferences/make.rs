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
pub struct MakePreferencesNode {}

impl MakePreferencesNode {
    pub fn new() -> Self {
        MakePreferencesNode {}
    }
}

#[async_trait]
impl NodeLogic for MakePreferencesNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "ai_generative_make_preferences",
            "Make Preferences",
            "Creates Model Preferences for model selection",
            "AI/Generative/Preferences",
        );
        node.add_icon("/flow/icons/struct.svg");

        node.add_output_pin(
            "preferences",
            "Preferences",
            "BitModelPreference",
            VariableType::Struct,
        )
        .set_schema::<BitModelPreference>();

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) ->flow_like_types::Result<()> {
        let preferences = BitModelPreference::default();

        context
            .set_pin_value("preferences", json!(preferences))
            .await?;

        Ok(())
    }
}
