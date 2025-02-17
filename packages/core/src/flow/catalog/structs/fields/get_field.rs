use std::sync::Arc;

use crate::{
    flow::{
        board::Board,
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        variable::VariableType,
    },
    state::FlowLikeState,
};
use ahash::HashMap;
use async_trait::async_trait;

#[derive(Default)]
pub struct GetStructFieldNode {}

impl GetStructFieldNode {
    pub fn new() -> Self {
        GetStructFieldNode {}
    }
}

#[async_trait]
impl NodeLogic for GetStructFieldNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "struct_get",
            "Get Field",
            "Fetches a field from a struct",
            "Structs/Fields",
        );
        node.add_icon("/flow/icons/struct.svg");

        node.add_output_pin(
            "value",
            "Value",
            "Value of the Struct",
            VariableType::Generic,
        );
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

    async fn run(&mut self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        let struct_value = context
            .evaluate_pin::<HashMap<String, serde_json::Value>>("struct")
            .await?;
        context.log_message(
            &format!("Got Value: {:?}", struct_value),
            crate::flow::execution::LogLevel::Debug,
        );
        let field = context.evaluate_pin::<String>("field").await?;

        let value = struct_value.get(&field);
        context
            .set_pin_value("found", serde_json::json!(value.is_some()))
            .await?;

        if let Some(value) = value {
            context.set_pin_value("value", value.clone()).await?;
            return Ok(());
        }

        context
            .set_pin_value("value", serde_json::Value::Null)
            .await?;

        return Ok(());
    }

    async fn on_update(&self, node: &mut Node, board: Arc<Board>) {
        let match_type = node.match_type("value", board, None);

        if match_type.is_err() {
            eprintln!("Error: {:?}", match_type.err());
        }
    }
}
