use std::{collections::{HashMap, HashSet}, sync::Arc};
use flow_like::{flow::{board::Board, execution::{context::ExecutionContext, internal_node::InternalNode, log::LogMessage, LogLevel}, node::{Node, NodeLogic}, pin::{PinOptions, ValueType}, variable::{Variable, VariableType}}, state::FlowLikeState};
use flow_like_types::{async_trait, json::json, reqwest, sync::{DashMap, Mutex}, Value};

use crate::{storage::path::FlowPath, web::api::{HttpBody, HttpRequest, HttpResponse, Method}};

#[derive(Default)]
pub struct StringEndsWithNode {}

impl StringEndsWithNode {
    pub fn new() -> Self {
        StringEndsWithNode {}
    }
}

#[async_trait]
impl NodeLogic for StringEndsWithNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "string_ends_with",
            "Ends With",
            "Checks if a string ends with a specific string",
            "Utils/String",
        );
        node.add_icon("/flow/icons/string.svg");

        node.add_input_pin("string", "String", "Input String", VariableType::String);
        node.add_input_pin(
            "suffix",
            "Suffix",
            "String to check against",
            VariableType::String,
        );

        node.add_output_pin(
            "ends_with",
            "Ends With?",
            "Does the string end with the suffix?",
            VariableType::Boolean,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) ->flow_like_types::Result<()> {
        let string: String = context.evaluate_pin("string").await?;
        let suffix: String = context.evaluate_pin("suffix").await?;

        let ends_with = string.ends_with(&suffix);

        context.set_pin_value("ends_with", json!(ends_with)).await?;
        Ok(())
    }
}
