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
use futures::StreamExt;
use object_store::PutPayload;

#[derive(Default)]
pub struct CopyNode {}

impl CopyNode {
    pub fn new() -> Self {
        CopyNode {}
    }
}

#[async_trait]
impl NodeLogic for CopyNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "storage_copy",
            "Copy",
            "Copies a file from one location to another",
            "Storage/Paths/Operations",
        );
        node.add_icon("/flow/icons/path.svg");

        node.add_input_pin(
            "exec_in",
            "Input",
            "Initiate Execution",
            VariableType::Execution,
        );

        node.add_input_pin("from", "From", "Source Path", VariableType::Struct)
            .set_schema::<FlowPath>()
            .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_input_pin("to", "To", "Destination Path", VariableType::Struct)
            .set_schema::<FlowPath>()
            .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_output_pin(
            "exec_out",
            "Success",
            "Execution if copy succeeds",
            VariableType::Execution,
        );
        node.add_output_pin(
            "exec_out_failure",
            "Failure",
            "Execution if copy fails",
            VariableType::Execution,
        );

        return node;
    }

    async fn run(&mut self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;
        context.activate_exec_pin("exec_out_failure").await?;

        let from_path: FlowPath = context.evaluate_pin("from").await?;
        let to_path: FlowPath = context.evaluate_pin("to").await?;

        let from_runtime = from_path.to_runtime(context).await?;
        let to_runtime = to_path.to_runtime(context).await?;

        if from_runtime.hash == to_runtime.hash {
            from_runtime
                .store
                .as_generic()
                .copy(&from_runtime.path, &to_runtime.path)
                .await?;
        } else {
            let bytes = from_runtime
                .store
                .as_generic()
                .get(&from_runtime.path)
                .await?;
            let mut response_stream = bytes.into_stream();
            let mut upload_stream = to_runtime
                .store
                .as_generic()
                .put_multipart(&to_runtime.path)
                .await?;
            while let Some(data) = response_stream.next().await {
                if let Ok(data) = data {
                    upload_stream.put_part(PutPayload::from_bytes(data)).await?;
                } else {
                    return Err(anyhow::anyhow!("Error reading source data"));
                }
            }
            upload_stream.complete().await?;
        };

        context.activate_exec_pin("exec_out").await?;
        context.deactivate_exec_pin("exec_out_failure").await?;

        Ok(())
    }
}
