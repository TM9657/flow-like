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
pub struct SetModelHintNode {}

impl SetModelHintNode {
    pub fn new() -> Self {
        SetModelHintNode {}
    }
}

#[async_trait]
impl NodeLogic for SetModelHintNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "ai_generative_set_model_hint",
            "Set Model Hint",
            "Sets the model hint in BitModelPreference",
            "AI/Generative/Preferences",
        );
        node.add_icon("/flow/icons/struct.svg");

        node.add_input_pin(
            "exec_in",
            "Input",
            "Initiate Execution",
            VariableType::Execution,
        );

        node.add_input_pin(
            "preferences_in",
            "Preferences",
            "Current Preferences",
            VariableType::Struct,
        )
        .set_schema::<BitModelPreference>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_input_pin(
            "model_hint",
            "Model Hint",
            "Model Hint to set",
            VariableType::String,
        );

        node.add_output_pin(
            "exec_out",
            "Output",
            "Done with the Execution",
            VariableType::Execution,
        );

        node.add_output_pin(
            "preferences_out",
            "Preferences",
            "Updated Preferences",
            VariableType::Struct,
        )
        .set_schema::<BitModelPreference>();

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) ->flow_like_types::Result<()> {
        let mut preferences: BitModelPreference = context.evaluate_pin("preferences_in").await?;
        let model_hint: String = context.evaluate_pin("model_hint").await?;

        preferences.model_hint = Some(model_hint);

        context
            .set_pin_value("preferences_out", json!(preferences))
            .await?;
        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }
}
