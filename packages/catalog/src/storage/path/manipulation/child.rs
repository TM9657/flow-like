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
pub struct ChildNode {}

impl ChildNode {
    pub fn new() -> Self {
        ChildNode {}
    }
}

#[async_trait]
impl NodeLogic for ChildNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "child",
            "Child",
            "Creates a child path from a parent path",
            "Storage/Paths/Path",
        );
        node.add_icon("/flow/icons/path.svg");

        node.add_input_pin(
            "exec_in",
            "Input",
            "Initiate Execution",
            VariableType::Execution,
        );

        node.add_input_pin(
            "parent_path",
            "Path",
            "Parent FlowPath",
            VariableType::Struct,
        )
        .set_schema::<FlowPath>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_input_pin(
            "child_name",
            "Child",
            "Name of the child",
            VariableType::String,
        );

        node.add_output_pin(
            "exec_out",
            "Output",
            "Done with the Execution",
            VariableType::Execution,
        );

        node.add_output_pin("path", "Path", "Child Path", VariableType::Struct)
            .set_schema::<FlowPath>();

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let parent_path: FlowPath = context.evaluate_pin("parent_path").await?;
        let child_name: String = context.evaluate_pin("child_name").await?;

        let mut path = parent_path.to_runtime(context).await?;
        path.path = path.path.child(child_name);
        let path = path.serialize().await;

        context.set_pin_value("path", json!(path)).await?;
        context.activate_exec_pin("exec_out").await?;
        Ok(())
    }
}
