use std::{collections::{HashMap, HashSet}, sync::Arc};
use flow_like::{flow::{board::Board, execution::{context::ExecutionContext, internal_node::InternalNode, log::LogMessage, LogLevel}, node::{Node, NodeLogic}, pin::{PinOptions, PinType, ValueType}, variable::{Variable, VariableType}}, state::FlowLikeState};
use flow_like_types::{async_trait, json::{from_str, json}, reqwest, sync::{DashMap, Mutex}, Value};
use regex::Regex;

use crate::{storage::path::FlowPath, web::api::{HttpBody, HttpRequest, HttpResponse, Method}};

#[derive(Default)]
pub struct FromStringNode {}

impl FromStringNode {
    pub fn new() -> Self {
        FromStringNode {}
    }
}

#[async_trait]
impl NodeLogic for FromStringNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "val_from_string",
            "From String",
            "Convert String to Struct",
            "Utils/Conversions",
        );
        node.add_icon("/flow/icons/convert.svg");

        node.add_input_pin(
            "string",
            "String",
            "String to convert",
            VariableType::String,
        );

        node.add_output_pin(
            "value",
            "Value",
            "Value of the Generic",
            VariableType::Generic,
        );
        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) ->flow_like_types::Result<()> {
        let string: String = context.evaluate_pin("string").await?;
        let value: Value = from_str(&string)?;
        context.set_pin_value("value", value).await?;
        Ok(())
    }

    async fn on_update(&self, node: &mut Node, board: Arc<Board>) {
        let mut value_type = ValueType::Normal;
        let var_type = node.get_pin_by_name("type").unwrap().default_value.clone();

        if let Some(var_type) = var_type {
            let parsed: Value = flow_like_types::json::from_slice(&var_type).unwrap();
            let parsed: String = flow_like_types::json::from_value(parsed).unwrap();
            match parsed.as_str() {
                "Normal" => value_type = ValueType::Normal,
                "Array" => value_type = ValueType::Array,
                "HashSet" => value_type = ValueType::HashSet,
                "HashMap" => value_type = ValueType::HashMap,
                _ => value_type = ValueType::Normal,
            }
        }

        let match_type = node.match_type("value", board, Some(value_type), None);

        if match_type.is_err() {
            eprintln!("Error: {:?}", match_type.err());
        }
    }
}
