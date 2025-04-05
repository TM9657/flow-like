use std::{collections::{HashMap, HashSet}, sync::Arc};
use flow_like::{flow::{board::Board, execution::{context::ExecutionContext, internal_node::InternalNode, log::LogMessage, LogLevel}, node::{Node, NodeLogic}, pin::{PinOptions, ValueType}, variable::{Variable, VariableType}}, state::FlowLikeState};
use flow_like_types::{async_trait, json::json, reqwest, sync::{DashMap, Mutex}, Value};

use crate::{storage::path::FlowPath, web::api::{HttpBody, HttpRequest, HttpResponse, Method}};

#[derive(Default)]
pub struct EqualStringNode {}

impl EqualStringNode {
    pub fn new() -> Self {
        EqualStringNode {}
    }
}

#[async_trait]
impl NodeLogic for EqualStringNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new("equal_string", "==", "Compares two Strings", "Utils/String");
        node.add_icon("/flow/icons/string.svg");

        node.add_input_pin("string", "String", "Input", VariableType::String);
        node.add_input_pin("string", "String", "Input", VariableType::String);

        node.add_output_pin(
            "equal",
            "Is Equal?",
            "Are the strings equal?",
            VariableType::Boolean,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) ->flow_like_types::Result<()> {
        let string_pins = context.get_pins_by_name("string").await?;
        let mut equal = true;
        let mut value = None;

        for pin in string_pins {
            let pin: String = context.evaluate_pin_ref(pin).await?;
            if let Some(value) = &value {
                if value != &pin {
                    equal = false;
                    break;
                }

                continue;
            }

            value = Some(pin);
        }

        context.set_pin_value("equal", json!(equal)).await?;
        Ok(())
    }
}
