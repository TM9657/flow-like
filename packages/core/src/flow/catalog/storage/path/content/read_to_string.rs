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
pub struct ReadToStringNode {}

impl ReadToStringNode {
    pub fn new() -> Self {
        ReadToStringNode {}
    }
}

#[async_trait]
impl NodeLogic for ReadToStringNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "read_to_string",
            "Read to String",
            "Reads the content of a file to a string",
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

        node.add_output_pin(
            "exec_out",
            "Output",
            "Done with the Execution",
            VariableType::Execution,
        );

        node.add_output_pin(
            "content",
            "Content",
            "The content of the file as a string",
            VariableType::String,
        );

        node.add_output_pin(
            "failed",
            "Failed",
            "Triggered if reading the file fails",
            VariableType::Execution,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        context.activate_exec_pin("failed").await?;
        context.deactivate_exec_pin("exec_out").await?;

        let path: FlowPath = context.evaluate_pin("path").await?;

        let path = path.to_runtime(context).await?;
        let store = path.store.as_generic();
        let content = store.get(&path.path).await?;

        let content = content.bytes().await?;

        let content = String::from_utf8_lossy(&content);
        let content = content.to_string();

        context.set_pin_value("content", json!(content)).await?;
        context.deactivate_exec_pin("failed").await?;
        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }
}
