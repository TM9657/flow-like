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
use bytes::Bytes;
use object_store::PutPayload;

#[derive(Default)]
pub struct WriteBytesNode {}

impl WriteBytesNode {
    pub fn new() -> Self {
        WriteBytesNode {}
    }
}

#[async_trait]
impl NodeLogic for WriteBytesNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "write_bytes",
            "Write Bytes",
            "Writes bytes to a file",
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

        node.add_input_pin(
            "content",
            "Content",
            "The content to write as bytes",
            VariableType::Byte,
        )
        .set_value_type(crate::flow::pin::ValueType::Array);

        node.add_output_pin(
            "exec_out",
            "Output",
            "Done with the Execution",
            VariableType::Execution,
        );

        node.add_output_pin(
            "failed",
            "Failed",
            "Triggered if writing the file fails",
            VariableType::Execution,
        );

        return node;
    }

    async fn run(&mut self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        context.activate_exec_pin("failed").await?;
        context.deactivate_exec_pin("exec_out").await?;

        let path: FlowPath = context.evaluate_pin("path").await?;
        let content: Vec<u8> = context.evaluate_pin("content").await?;

        let path = path.to_runtime(context).await?;
        let store = path.store.as_generic();
        let bytes = Bytes::from(content);
        let payload = PutPayload::from_bytes(bytes);

        store.put(&path.path, payload).await?;

        context.deactivate_exec_pin("failed").await?;
        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }
}