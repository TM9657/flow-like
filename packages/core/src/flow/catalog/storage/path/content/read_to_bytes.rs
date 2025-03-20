use crate::{
    flow::{
        catalog::storage::path::FlowPath,
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        pin::PinOptions,
        variable::VariableType,
    },
    state::FlowLikeState,
};
use async_trait::async_trait;
use serde_json::json;

#[derive(Default)]
pub struct ReadToBytesNode {}

impl ReadToBytesNode {
    pub fn new() -> Self {
        ReadToBytesNode {}
    }
}

#[async_trait]
impl NodeLogic for ReadToBytesNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "read_to_bytes",
            "Read to Bytes",
            "Reads the content of a file to bytes",
            "Storage/Paths/Content",
        );
        node.add_icon("/flow/icons/binary.svg"); // Consider a more appropriate icon

        node.add_input_pin(
            "exec_in",
            "Input",
            "Initiate Execution",
            VariableType::Execution,
        );

        node.add_input_pin("path", "Path", "FlowPath", VariableType::Struct)
            .set_schema::<FlowPath>()
            .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_output_pin(
            "exec_out",
            "Output",
            "Done with the Execution",
            VariableType::Execution,
        );

        node.add_output_pin(
            "content",
            "Content",
            "The content of the file as bytes",
            VariableType::Byte,
        )
        .set_value_type(crate::flow::pin::ValueType::Array);

        node.add_output_pin(
            "failed",
            "Failed",
            "Triggered if reading the file fails",
            VariableType::Execution,
        );

        return node;
    }

    async fn run(&mut self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        context.activate_exec_pin("failed").await?;
        context.deactivate_exec_pin("exec_out").await?;

        let path: FlowPath = context.evaluate_pin("path").await?;

        let path = path.to_runtime(context).await?;
        let store = path.store.as_generic();
        let content = store.get(&path.path).await?;

        let content = content.bytes().await?;
        let bytes = content.to_vec();


        context.set_pin_value("content", json!(bytes)).await?;
        context.deactivate_exec_pin("failed").await?;
        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }
}