use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use flow_like::{
    flow::{
        board::Board,
        execution::{LogLevel, context::ExecutionContext, internal_node::InternalNode},
        node::{Node, NodeLogic},
        pin::PinType,
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_types::{Value, async_trait, json::from_slice};

#[derive(Default)]
pub struct CallReferenceNode {}

impl CallReferenceNode {
    pub fn new() -> Self {
        CallReferenceNode {}
    }
}

#[async_trait]
impl NodeLogic for CallReferenceNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "control_call_reference",
            "Call Reference",
            "References a specific call in the flow",
            "Control/Call",
        );
        node.add_icon("/flow/icons/workflow.svg");

        node.add_input_pin("exec_in", "Input", "Trigger Pin", VariableType::Execution);
        node.add_input_pin(
            "fn_ref",
            "Function Reference",
            "The function reference to call",
            VariableType::String,
        );

        node.add_output_pin(
            "exec_out",
            "Done",
            "The flow to follow if the function call is successful",
            VariableType::Execution,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;
        let fn_ref: String = context.evaluate_pin("fn_ref").await?;

        let mut content_pins = HashMap::with_capacity(context.node.pins.len());
        let input_pins = context.node.pins.clone();

        for (_id, pin) in input_pins {
            let value = context.evaluate_pin_ref::<Value>(pin.clone()).await;
            let name = pin.lock().await.pin.lock().await.name.clone();
            if let Ok(value) = value {
                content_pins.insert(name, value);
                continue;
            }

            context.log_message(
                &format!("Failed to evaluate pin {}: {:?}", name, value),
                LogLevel::Warn,
            );
        }

        let reference_function = context
            .nodes
            .get(&fn_ref)
            .ok_or_else(|| flow_like_types::anyhow!("Function reference not found"))?;

        let node_ref = reference_function.node.clone();

        let pins = reference_function.pins.clone();
        for (_id, pin) in pins {
            let guard = pin.lock().await;
            let (pin_type, data_type, name) = {
                let pin = guard.pin.lock().await;
                (
                    pin.pin_type.clone(),
                    pin.data_type.clone(),
                    pin.name.clone(),
                )
            };
            if pin_type == PinType::Input || data_type == VariableType::Execution {
                continue;
            }

            if let Some(value) = content_pins.get(&name) {
                guard.set_value(value.clone()).await;
            }
        }

        let mut sub_context = context.create_sub_context(reference_function).await;
        sub_context.delegated = true;
        let run = InternalNode::trigger(&mut sub_context, &mut None, true).await;
        sub_context.end_trace();
        context.push_sub_context(sub_context);

        if run.is_err() {
            let node_name = node_ref.lock().await.friendly_name.clone();
            let error = run.err().unwrap();
            context.log_message(
                &format!("Error: {:?} calling function {}", error, node_name),
                LogLevel::Error,
            );
        }

        context.activate_exec_pin("exec_out").await?;
        return Ok(());
    }

    async fn on_update(&self, node: &mut Node, board: Arc<Board>) {
        node.error = None;
        let node_ref = match node.get_pin_by_name("fn_ref") {
            Some(pin) => pin.clone(),
            None => {
                node.error = Some("Function reference pin not found".to_string());
                return;
            }
        };

        let reference = match node_ref.default_value {
            Some(value) => value,
            None => {
                node.error = Some("Function reference pin is not connected".to_string());
                return;
            }
        };

        let reference = match from_slice::<String>(&reference) {
            Ok(value) => value,
            Err(err) => {
                node.error = Some(format!("Failed to parse function reference: {}", err));
                return;
            }
        };

        let event = match board.nodes.get(&reference) {
            Some(event) => event.clone(),
            None => {
                node.error = Some(format!("Function reference not found: {}", reference));
                return;
            }
        };

        node.friendly_name = format!("Call {}", event.friendly_name);
        node.description = format!("Calls the function {}", event.friendly_name);
        node.icon = event.icon.clone();

        let mut output_pins = event
            .pins
            .iter()
            .filter(|pin| {
                pin.1.pin_type == PinType::Output && pin.1.data_type != VariableType::Execution
            })
            .map(|pin| {
                let mut pin = pin.1.clone();
                pin.index += 1;
                pin
            })
            .collect::<Vec<_>>();

        output_pins.sort_by(|a, b| a.index.cmp(&b.index));
        let mut relevant_pins = HashSet::with_capacity(output_pins.len());
        for pin in output_pins {
            relevant_pins.insert(pin.name.clone());
            if node.pins.iter().any(|(_, p)| p.name == pin.name) {
                continue;
            }
            let new_pin = node.add_input_pin(
                &pin.name,
                &pin.friendly_name,
                &pin.description,
                pin.data_type,
            );
            new_pin.schema = pin.schema.clone();
            new_pin.options = pin.options.clone();
        }
        node.pins.retain(|_, pin| {
            if pin.pin_type == PinType::Input && pin.data_type != VariableType::Execution {
                relevant_pins.contains(&pin.name) || pin.name == "fn_ref"
            } else {
                true
            }
        });
    }
}
