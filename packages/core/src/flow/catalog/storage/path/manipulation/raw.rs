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
pub struct RawPathNode {}

impl RawPathNode {
    pub fn new() -> Self {
        RawPathNode {}
    }
}

#[async_trait]
impl NodeLogic for RawPathNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "raw_path",
            "Raw Path",
            "Gets the raw path string",
            "Storage/Paths/Path",
        );
        node.add_icon("/flow/icons/path.svg");

        node.add_input_pin("path", "Path", "FlowPath", VariableType::Struct)
            .set_schema::<FlowPath>()
            .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_output_pin(
            "raw_path",
            "Raw Path",
            "Raw Path String",
            VariableType::String,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        let path: FlowPath = context.evaluate_pin("path").await?;

        let path = path.to_runtime(context).await?;
        let raw_path = path.path.as_ref().to_string();

        context.set_pin_value("raw_path", json!(raw_path)).await?;
        Ok(())
    }
}
