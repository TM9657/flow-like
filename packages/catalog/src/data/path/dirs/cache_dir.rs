use crate::data::path::FlowPath;
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
pub struct PathFromCacheDirNode {}

impl PathFromCacheDirNode {
    pub fn new() -> Self {
        PathFromCacheDirNode {}
    }
}

#[async_trait]
impl NodeLogic for PathFromCacheDirNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "path_from_cache_dir",
            "Cache Dir",
            "Converts the cache directory to a Path",
            "Data/Files/Directories",
        );
        node.add_icon("/flow/icons/path.svg");

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

        node.add_output_pin("path", "Path", "Output Path", VariableType::Struct)
            .set_schema::<FlowPath>();

        node.add_input_pin(
            "node_scope",
            "Node Scope",
            "Is this node in the node scope?",
            VariableType::Boolean,
        )
        .set_default_value(Some(json!(false)));

        node.add_input_pin(
            "user_scope",
            "User Scope",
            "Store in user context?",
            VariableType::Boolean,
        )
        .set_default_value(Some(json!(false)));

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;

        let node_scope: bool = context.evaluate_pin("node_scope").await?;
        let user_scope: bool = context.evaluate_pin("user_scope").await?;

        let path = FlowPath::from_cache_dir(context, node_scope, user_scope).await?;
        context.set_pin_value("path", json!(path)).await?;

        context.activate_exec_pin("exec_out").await?;
        Ok(())
    }
}
