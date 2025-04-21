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
use flow_like_types::{async_trait, json::json};

#[derive(Default)]
pub struct SetExtensionNode {}

impl SetExtensionNode {
    pub fn new() -> Self {
        SetExtensionNode {}
    }
}

#[async_trait]
impl NodeLogic for SetExtensionNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "set_extension",
            "Set Extension",
            "Sets the file extension of a path",
            "Storage/Paths/Path",
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
            "extension",
            "Extension",
            "New File Extension",
            VariableType::String,
        );

        node.add_output_pin(
            "exec_out",
            "Output",
            "Done with the Execution",
            VariableType::Execution,
        );

        node.add_output_pin(
            "path_out",
            "Path",
            "Modified FlowPath",
            VariableType::Struct,
        )
        .set_schema::<FlowPath>();

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let path: FlowPath = context.evaluate_pin("path").await?;
        let extension: String = context.evaluate_pin("extension").await?;
        let path = path.set_extension(context, &extension).await?;

        context.set_pin_value("path_out", json!(path)).await?;
        context.activate_exec_pin("exec_out").await?;
        Ok(())
    }
}
