use std::sync::Arc;

use flow_like::{
    flow::{
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_types::{async_trait, json::json};
pub mod push_generic_result;

#[crate::register_node]
#[derive(Default)]
pub struct GenericEventNode {}

impl GenericEventNode {
    pub fn new() -> Self {
        GenericEventNode {}
    }
}

#[async_trait]
impl NodeLogic for GenericEventNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "events_discord",
            "Discord Event",
            "A generic event without input or output",
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

        node.add_output_pin(
            "payload",
            "Payload",
            "The payload of the event",
            VariableType::Struct,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let exec_out_pin = context.get_pin_by_name("exec_out").await?;

        if context.delegated {
            context.activate_exec_pin_ref(&exec_out_pin).await?;
            return Ok(());
        }

        let payload_data = context.get_payload().await?;
        let mut payload = payload_data
            .payload
            .clone()
            .ok_or_else(|| flow_like_types::anyhow!("Payload is missing"))?;

        println!("Generic Event triggered with payload: {}", payload);

        if let Some(obj) = payload.as_object_mut() {
            let mut output_pins = Vec::new();
            for (_, pin_ref) in context.node.pins.iter() {
                let pin_ref_guard = pin_ref.lock().await;
                let pin = pin_ref_guard.pin.lock().await;
                if pin.pin_type == flow_like::flow::pin::PinType::Output
                    && pin.data_type != VariableType::Execution
                    && pin.name != "payload"
                {
                    output_pins.push(pin.name.clone());
                }
            }

            for pin_name in output_pins {
                if let Some(value) = obj.remove(&pin_name) {
                    context.set_pin_value(&pin_name, value).await?;
                }
            }

            context.set_pin_value("payload", json!(obj)).await?;
        } else {
            context.set_pin_value("payload", payload).await?;
        }

        context.activate_exec_pin_ref(&exec_out_pin).await?;

        return Ok(());
    }
}