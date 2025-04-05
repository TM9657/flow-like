use std::{collections::{HashMap, HashSet}, sync::Arc};
use flow_like::{flow::{board::Board, execution::{context::ExecutionContext, internal_node::InternalNode, log::LogMessage, LogLevel}, node::{Node, NodeLogic}, pin::{PinOptions, ValueType}, variable::{Variable, VariableType}}, state::FlowLikeState};
use flow_like_types::{async_trait, json::json, reqwest, sync::{DashMap, Mutex}, Value};

use crate::{storage::path::FlowPath, web::api::{HttpBody, HttpRequest, HttpResponse, Method}};

#[derive(Default)]
pub struct StringTrimNode {}

impl StringTrimNode {
    pub fn new() -> Self {
        StringTrimNode {}
    }
}

#[async_trait]
impl NodeLogic for StringTrimNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "string_trim",
            "Trim String",
            "Removes leading and trailing whitespace from a string",
            "Utils/String",
        );
        node.add_icon("/flow/icons/string.svg");

        node.add_input_pin("string", "String", "Input String", VariableType::String);

        node.add_output_pin(
            "trimmed_string",
            "Trimmed String",
            "String without leading/trailing whitespace",
            VariableType::String,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) ->flow_like_types::Result<()> {
        let string: String = context.evaluate_pin("string").await?;
        let trimmed_string = string.trim().to_string();

        context
            .set_pin_value("trimmed_string", json!(trimmed_string))
            .await?;
        Ok(())
    }
}
