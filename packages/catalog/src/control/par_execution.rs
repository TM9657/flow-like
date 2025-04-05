use std::{collections::{HashMap, HashSet}, sync::Arc, time::Duration};
use flow_like::{flow::{board::Board, execution::{context::ExecutionContext, internal_node::InternalNode, log::LogMessage, LogLevel}, node::{Node, NodeLogic}, pin::{PinOptions, PinType, ValueType}, variable::{Variable, VariableType}}, state::{FlowLikeState, ToastLevel}};
use flow_like_types::{async_trait, json::{json, Deserialize, Serialize}, reqwest, sync::{DashMap, Mutex}, Bytes, Error, JsonSchema, Value};
use nalgebra::DVector;
use regex::Regex;
use flow_like_storage::{object_store::PutPayload, Path};
use futures::StreamExt;
use crate::{storage::path::FlowPath, web::api::{HttpBody, HttpRequest, HttpResponse, Method}};

#[derive(Default)]
pub struct ParallelExecutionNode {}

impl ParallelExecutionNode {
    pub fn new() -> Self {
        ParallelExecutionNode {}
    }
}

#[async_trait]
impl NodeLogic for ParallelExecutionNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "control_par_execution",
            "Parallel Execution",
            "Parallel Execution",
            "Control",
        );
        node.add_icon("/flow/icons/par_execution.svg");

        node.add_input_pin("exec_in", "Input", "Trigger Pin", VariableType::Execution);
        node.add_output_pin("exec_out", "Output", "Output Pin", VariableType::Execution);
        node.add_output_pin("exec_out", "Output", "Output Pin", VariableType::Execution);

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) ->flow_like_types::Result<()> {
        let exec_out_pins = context.get_pins_by_name("exec_out").await?;
        for pin in exec_out_pins {
            let deactivate_pin = context.activate_exec_pin_ref(&pin).await;
            if let Err(err) = deactivate_pin {
                eprintln!("Error activating pin: {:?}", err);
            }
        }

        return Ok(());
    }
}
