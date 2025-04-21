use crate::storage::path::FlowPath;
use flow_like::{
    flow::{
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_types::{async_trait, json::json};

#[derive(Default)]
pub struct PathFromUploadDirNode {}

impl PathFromUploadDirNode {
    pub fn new() -> Self {
        PathFromUploadDirNode {}
    }
}

#[async_trait]
impl NodeLogic for PathFromUploadDirNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "path_from_upload_dir",
            "Upload Dir",
            "Converts the upload directory to a Path",
            "Storage/Paths/Directories",
        );
        node.add_icon("/flow/icons/path.svg");

        node.add_output_pin("path", "Path", "Output Path", VariableType::Struct)
            .set_schema::<FlowPath>();

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let path = FlowPath::from_upload_dir(context).await?;
        context.set_pin_value("path", json!(path)).await?;

        let _ = context.activate_exec_pin("exec_out").await;
        Ok(())
    }
}
