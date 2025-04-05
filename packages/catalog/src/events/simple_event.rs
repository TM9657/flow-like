use std::{collections::{HashMap, HashSet}, sync::Arc, time::Duration};
use flow_like::{flow::{board::Board, execution::{context::ExecutionContext, internal_node::InternalNode, log::LogMessage, LogLevel}, node::{Node, NodeLogic}, pin::{PinOptions, PinType, ValueType}, variable::{Variable, VariableType}}, state::{FlowLikeState, ToastLevel}};
use flow_like_types::{async_trait, json::{json, Deserialize, Serialize}, reqwest, sync::{DashMap, Mutex}, Bytes, Error, JsonSchema, Value};
use nalgebra::DVector;
use regex::Regex;
use flow_like_storage::{object_store::PutPayload, Path};
use futures::StreamExt;
use crate::{storage::path::FlowPath, web::api::{HttpBody, HttpRequest, HttpResponse, Method}};

#[derive(Default)]
pub struct SimpleEventNode {}

impl SimpleEventNode {
    pub fn new() -> Self {
        SimpleEventNode {}
    }
}

#[async_trait]
impl NodeLogic for SimpleEventNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "events_simple",
            "Simple Event",
            "A simple event without input or output",
            "Events",
        );
        node.add_icon("/flow/icons/event.svg");
        node.set_start(true);

        node.add_output_pin(
            "exec_out",
            "Output",
            "Starting an event",
            VariableType::Execution,
        );
        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) ->flow_like_types::Result<()> {
        let exec_out_pin = context.get_pin_by_name("exec_out").await?;

        context.activate_exec_pin_ref(&exec_out_pin).await?;

        return Ok(());
    }
}
