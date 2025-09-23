use flow_like::{
    flow::{
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_types::async_trait;
use std::collections::HashMap;

#[derive(Default)]
pub struct ListStructFields {}

impl ListStructFields {
    pub fn new() -> Self {
        ListStructFields {}
    }
}

#[async_trait]
impl NodeLogic for ListStructFields {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "struct_get_fields",
            "Get Fields",
            "Fetches fields from a struct",
            "Structs/Fields",
        );
        node.add_icon("/flow/icons/struct.svg");

        node.add_input_pin("struct", "Struct", "Struct Output", VariableType::Struct);

        node.add_output_pin("field_names", "Field Names", "Fields", VariableType::String)
            .set_value_type(flow_like::flow::pin::ValueType::Array)
            .set_default_value(Some(flow_like_types::json::json!([])));

        node.add_output_pin("fields", "Fields", "Fields", VariableType::Generic)
            .set_value_type(flow_like::flow::pin::ValueType::Array)
            .set_default_value(Some(flow_like_types::json::json!([])));

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let struct_value = context.evaluate_pin::<HashMap<String, flow_like_types::Value>>("struct").await?;

        context.set_pin_value("field_names", flow_like_types::json::json!(struct_value.keys().cloned().collect::<Vec<_>>())).await?;
        context.set_pin_value("fields", flow_like_types::json::json!(struct_value.values().cloned().collect::<Vec<_>>())).await?;

        return Ok(());
    }
}
