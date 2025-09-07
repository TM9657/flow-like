use crate::data::path::FlowPath;
use flow_like::{
    flow::{
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        pin::{PinOptions, ValueType},
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_types::{async_trait, json::json};
use std::ops::Range;

#[derive(Default)]
pub struct GetRangeNode {}

impl GetRangeNode {
    pub fn new() -> Self {
        GetRangeNode {}
    }
}

#[async_trait]
impl NodeLogic for GetRangeNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "path_get_range",
            "Get Range",
            "Reads a range of bytes from a file",
            "Data/Files/Operations",
        );
        node.add_icon("/flow/icons/path.svg");

        node.add_input_pin(
            "exec_in",
            "Input",
            "Initiate Execution",
            VariableType::Execution,
        );

        node.add_input_pin("path", "Path", "FlowPath", VariableType::Struct)
            .set_schema::<FlowPath>()
            .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_input_pin("from", "From", "Start of the Range", VariableType::Integer);

        node.add_input_pin("to", "To", "End of the Range", VariableType::Integer);

        node.add_output_pin(
            "exec_out",
            "Output",
            "Done with the Execution",
            VariableType::Execution,
        );

        node.add_output_pin("bytes", "Bytes", "Output Bytes", VariableType::Byte)
            .set_value_type(ValueType::Array);

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;
        let path: FlowPath = context.evaluate_pin("path").await?;
        let from: i64 = context.evaluate_pin("from").await?;
        let to: i64 = context.evaluate_pin("to").await?;
        let range: Range<usize> = from as usize..to as usize;

        let path = path.to_runtime(context).await?;
        let generic_store = path.store.as_generic();
        let bytes = generic_store.get_range(&path.path, range).await?;
        let bytes = bytes.to_vec();
        context.set_pin_value("bytes", json!(bytes)).await?;
        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }
}
