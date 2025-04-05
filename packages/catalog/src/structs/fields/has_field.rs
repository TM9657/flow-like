use std::{collections::{HashMap, HashSet}, sync::Arc};
use flow_like::{flow::{board::Board, execution::{context::ExecutionContext, internal_node::InternalNode, log::LogMessage, LogLevel}, node::{Node, NodeLogic}, pin::{PinOptions, PinType, ValueType}, variable::{Variable, VariableType}}, state::FlowLikeState};
use flow_like_types::{async_trait, json::json, reqwest, sync::{DashMap, Mutex}, Value};
use nalgebra::DVector;
use regex::Regex;

use crate::{storage::path::FlowPath, web::api::{HttpBody, HttpRequest, HttpResponse, Method}};

#[derive(Default)]
pub struct HasStructFieldNode {}

impl HasStructFieldNode {
    pub fn new() -> Self {
        HasStructFieldNode {}
    }
}

#[async_trait]
impl NodeLogic for HasStructFieldNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "struct_has",
            "Has Field",
            "Checks if a field exists in a struct",
            "Structs/Fields",
        );
        node.add_icon("/flow/icons/struct.svg");

        node.add_output_pin(
            "found",
            "Found?",
            "Indicates if the value was found",
            VariableType::Boolean,
        );

        node.add_input_pin("struct", "Struct", "Struct Output", VariableType::Struct);

        node.add_input_pin("field", "Field", "Field to get", VariableType::String);

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) ->flow_like_types::Result<()> {
        let struct_value = context
            .evaluate_pin::<HashMap<String, flow_like_types::Value>>("struct")
            .await?;
        let field = context.evaluate_pin::<String>("field").await?;

        let value = struct_value.get(&field);
        context
            .set_pin_value("found", flow_like_types::json::json!(value.is_some()))
            .await?;

        return Ok(());
    }
}
