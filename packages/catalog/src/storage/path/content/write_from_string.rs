use crate::storage::path::FlowPath;
use flow_like::{
    flow::{
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        pin::PinOptions,
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_storage::object_store::PutPayload;
use flow_like_types::{Bytes, async_trait};

#[derive(Default)]
pub struct WriteStringNode {}

impl WriteStringNode {
    pub fn new() -> Self {
        WriteStringNode {}
    }
}

#[async_trait]
impl NodeLogic for WriteStringNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "write_string",
            "Write String",
            "Writes a string to a file",
            "Storage/Paths/Content",
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

        node.add_input_pin(
            "content",
            "Content",
            "The content to write as a string",
            VariableType::String,
        );

        node.add_output_pin(
            "exec_out",
            "Output",
            "Done with the Execution",
            VariableType::Execution,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;

        let path: FlowPath = context.evaluate_pin("path").await?;
        let content: String = context.evaluate_pin("content").await?;

        let path = path.to_runtime(context).await?;
        let store = path.store.as_generic();
        let bytes = Bytes::from(content.into_bytes());
        let payload = PutPayload::from_bytes(bytes);

        store.put(&path.path, payload).await?;

        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }
}
