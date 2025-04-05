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
pub struct SetHistoryPresencePenaltyNode {}

impl SetHistoryPresencePenaltyNode {
    pub fn new() -> Self {
        SetHistoryPresencePenaltyNode {}
    }
}

#[async_trait]
impl NodeLogic for SetHistoryPresencePenaltyNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "ai_generative_set_history_presence_penalty",
            "Set History Presence Penalty",
            "Sets the presence_penalty attribute in a ChatHistory",
            "AI/Generative/History",
        );
        node.add_icon("/flow/icons/history.svg");

        node.add_input_pin(
            "exec_in",
            "Input",
            "Initiate Execution",
            VariableType::Execution,
        );

        node.add_input_pin("history", "History", "ChatHistory", VariableType::Struct)
            .set_schema::<History>()
            .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_input_pin(
            "presence_penalty",
            "Presence Penalty",
            "Presence Penalty Value",
            VariableType::Float,
        )
        .set_options(PinOptions::new().set_range((0.0, 1.0)).build());

        node.add_output_pin(
            "exec_out",
            "Output",
            "Done with the Execution",
            VariableType::Execution,
        );

        node.add_output_pin(
            "history_out",
            "History",
            "Updated ChatHistory",
            VariableType::Struct,
        )
        .set_schema::<History>();

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) ->flow_like_types::Result<()> {
        let mut history: History = context.evaluate_pin("history").await?;
        let presence_penalty: f64 = context.evaluate_pin("presence_penalty").await?;

        history.presence_penalty = Some(presence_penalty as f32);

        context.set_pin_value("history_out", json!(history)).await?;
        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }
}
