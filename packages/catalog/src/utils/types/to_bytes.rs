use std::{collections::{HashMap, HashSet}, sync::Arc};
use flow_like::{flow::{board::Board, execution::{context::ExecutionContext, internal_node::InternalNode, log::LogMessage, LogLevel}, node::{Node, NodeLogic}, pin::{PinOptions, PinType, ValueType}, variable::{Variable, VariableType}}, state::FlowLikeState};
use flow_like_types::{async_trait, json::json, reqwest, sync::{DashMap, Mutex}, Value};
use regex::Regex;

use crate::{storage::path::FlowPath, web::api::{HttpBody, HttpRequest, HttpResponse, Method}};

#[derive(Default)]
pub struct ToBytesNode {}

impl ToBytesNode {
    pub fn new() -> Self {
        ToBytesNode {}
    }
}

#[async_trait]
impl NodeLogic for ToBytesNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "val_to_bytes",
            "To Bytes",
            "Convert Struct to Bytes",
            "Utils/Conversions",
        );
        node.add_icon("/flow/icons/convert.svg");

        node.add_input_pin("value", "Value", "Input Value", VariableType::Generic);
        node.add_input_pin(
            "pretty",
            "Pretty?",
            "Should the struct be pretty printed?",
            VariableType::Boolean,
        );

        node.add_output_pin("bytes", "Bytes", "Output Bytes", VariableType::Byte)
            .set_value_type(ValueType::Array);

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) ->flow_like_types::Result<()> {
        let value: Value = context.evaluate_pin("value").await?;
        let pretty = context.evaluate_pin::<bool>("pretty").await?;
        let bytes: Vec<u8> = if pretty {
            flow_like_types::json::to_vec_pretty(&value)?
        } else {
            flow_like_types::json::to_vec(&value)?
        };
        context
            .set_pin_value("bytes", flow_like_types::json::json!(bytes))
            .await?;
        Ok(())
    }

    async fn on_update(&self, node: &mut Node, board: Arc<Board>) {
        let match_type = node.match_type("value", board, None, None);

        if match_type.is_err() {
            eprintln!("Error: {:?}", match_type.err());
        }
    }
}
