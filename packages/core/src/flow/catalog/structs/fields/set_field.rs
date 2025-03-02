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
pub struct SetStructFieldNode {}

impl SetStructFieldNode {
    pub fn new() -> Self {
        SetStructFieldNode {}
    }
}

#[async_trait]
impl NodeLogic for SetStructFieldNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "struct_set",
            "Set Field",
            "Sets a field in a struct",
            "Structs/Fields",
        );
        node.add_icon("/flow/icons/struct.svg");

        node.add_input_pin(
            "exec_in",
            "Input",
            "Initiate Execution",
            VariableType::Execution,
        );

        node.add_output_pin(
            "exec_out",
            "Output",
            "Done with the Execution",
            VariableType::Execution,
        );
        node.add_output_pin("struct_out", "Struct", "Struct Out", VariableType::Struct);
        node.add_input_pin("struct_in", "Struct", "Struct In", VariableType::Struct);

        node.add_input_pin("field", "Field", "Field to get", VariableType::String);

        node.add_input_pin("value", "Value", "Value to set", VariableType::Generic);

        return node;
    }

    async fn run(&mut self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        let mut old_struct = context
            .evaluate_pin::<HashMap<String, serde_json::Value>>("struct_in")
            .await?;
        let field = context.evaluate_pin::<String>("field").await?;
        let value = context.evaluate_pin::<serde_json::Value>("value").await?;
        old_struct.insert(field, value);
        context
            .set_pin_value("struct_out", serde_json::json!(old_struct))
            .await?;
        context.activate_exec_pin("exec_out").await?;
        return Ok(());
    }

    async fn on_update(&self, node: &mut Node, board: Arc<Board>) {
        let match_type = node.match_type("value", board, None, None);

        if match_type.is_err() {
            eprintln!("Error: {:?}", match_type.err());
        }
    }
}
