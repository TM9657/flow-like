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
pub struct PopHistoryMessageNode {}

impl PopHistoryMessageNode {
    pub fn new() -> Self {
        PopHistoryMessageNode {}
    }
}

#[async_trait]
impl NodeLogic for PopHistoryMessageNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "ai_generative_pop_history_message",
            "Pop Message from History",
            "Removes and returns the last message from a ChatHistory",
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

        node.add_output_pin(
            "message",
            "Message",
            "Removed Message",
            VariableType::Struct,
        )
        .set_schema::<HistoryMessage>();

        node.add_output_pin(
            "empty",
            "Empty",
            "History was empty",
            VariableType::Execution,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) ->flow_like_types::Result<()> {
        let mut history: History = context.evaluate_pin("history").await?;

        if history.messages.is_empty() {
            context.activate_exec_pin("empty").await?;
            context.set_pin_value("history_out", json!(history)).await?;
            return Ok(());
        }

        if let Some(message) = history.messages.pop() {
            context.set_pin_value("message", json!(message)).await?;
            context.set_pin_value("history_out", json!(history)).await?;
            context.activate_exec_pin("exec_out").await?;
        }

        Ok(())
    }
}
