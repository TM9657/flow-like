use flow_like::{
    flow::{
        board::Board,
        execution::{LogLevel, context::ExecutionContext},
        node::{Node, NodeLogic},
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_types::async_trait;
use std::{collections::HashMap, sync::Arc};

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

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let struct_value = context
            .evaluate_pin::<HashMap<String, flow_like_types::Value>>("struct")
            .await?;
        context.log_message(&format!("Got Value: {:?}", struct_value), LogLevel::Debug);
        let field = context.evaluate_pin::<String>("field").await?;

        let value = struct_value.get(&field);
        context
            .set_pin_value("found", flow_like_types::json::json!(value.is_some()))
            .await?;

        if let Some(value) = value {
            context.set_pin_value("value", value.clone()).await?;
            return Ok(());
        }

        context
            .set_pin_value("value", flow_like_types::Value::Null)
            .await?;

        return Ok(());
    }

    async fn on_update(&self, node: &mut Node, board: Arc<Board>) {
        let match_type = node.match_type("value", board, None, None);

        if match_type.is_err() {
            eprintln!("Error: {:?}", match_type.err());
        }
    }
}
