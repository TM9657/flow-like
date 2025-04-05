use std::{collections::{HashMap, HashSet}, sync::Arc, time::Duration};
use flow_like::{flow::{board::Board, execution::{context::ExecutionContext, internal_node::InternalNode, log::LogMessage, LogLevel}, node::{Node, NodeLogic}, pin::{PinOptions, PinType, ValueType}, variable::{Variable, VariableType}}, state::{FlowLikeState, ToastLevel}};
use flow_like_types::{async_trait, json::{json, Deserialize, Serialize}, reqwest, sync::{DashMap, Mutex}, Bytes, Error, JsonSchema, Value};
use nalgebra::DVector;
use regex::Regex;
use flow_like_storage::{object_store::PutPayload, Path};
use futures::StreamExt;
use crate::{storage::path::FlowPath, web::api::{HttpBody, HttpRequest, HttpResponse, Method}};

#[derive(Default)]
pub struct SequenceNode {}

impl SequenceNode {
    pub fn new() -> Self {
        SequenceNode {}
    }
}

#[async_trait]
impl NodeLogic for SequenceNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "control_sequence",
            "Sequence",
            "Sequential Execution",
            "Control",
        );
        node.add_icon("/flow/icons/sequence.svg");

        node.add_input_pin("exec_in", "Input", "Trigger Pin", VariableType::Execution);

        node.add_output_pin("exec_out", "Output", "Output Pin", VariableType::Execution);

        node.add_output_pin("exec_out", "Output", "Output Pin", VariableType::Execution);

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) ->flow_like_types::Result<()> {
        let mut already_executed = HashSet::new();

        let mut pin_indices = {
            let exec_out_pins = context.get_pins_by_name("exec_out").await?;
            let mut pin_indices = Vec::new();
            for pin in exec_out_pins {
                let index = pin.lock().await.pin.lock().await.index;
                pin_indices.push((pin.clone(), index));
            }
            pin_indices
        };

        pin_indices.sort_by_key(|(_, index)| *index);

        let execution_order = {
            let mut execution_order = Vec::with_capacity(pin_indices.len());

            for (pin, _) in pin_indices {
                let activate_pin = context.activate_exec_pin_ref(&pin).await;
                if let Err(err) = activate_pin {
                    eprintln!("Error activating pin: {:?}", err);
                }
                let pin = pin.lock().await;
                let connected_to = pin.connected_to.clone();
                for connection in connected_to {
                    let connection = connection
                        .upgrade()
                        .ok_or(flow_like_types::anyhow!("Connection not Valid"))?;
                    let connection = connection.lock().await;
                    let node = connection.node.upgrade();
                    if let Some(node) = node {
                        let node_id = node.node.lock().await.id.clone();
                        if !already_executed.contains(&node_id) {
                            execution_order.push((node.clone(), node_id.clone()));
                            already_executed.insert(node_id);
                        }
                    }
                }
            }
            execution_order
        };

        let mut recursion_guard = HashSet::new();
        recursion_guard.insert(context.node.node.lock().await.id.clone());

        for (node, _node_id) in execution_order {
            let mut sub_context = context.create_sub_context(&node).await;
            let _ =
                InternalNode::trigger(&mut sub_context, &mut Some(recursion_guard.clone()), true)
                    .await;
            sub_context.end_trace();
            context.push_sub_context(sub_context);
        }

        let exec_out_pins = context.get_pins_by_name("exec_out").await?;
        for pin in exec_out_pins {
            let deactivate_pin = context.deactivate_exec_pin_ref(&pin).await;
            if let Err(err) = deactivate_pin {
                eprintln!("Error deactivating pin: {:?}", err);
            }
        }

        return Ok(());
    }
}
