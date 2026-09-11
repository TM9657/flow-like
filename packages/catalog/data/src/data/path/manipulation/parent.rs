use crate::data::path::FlowPath;
use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic},
    pin::PinOptions,
    variable::VariableType,
};
use flow_like_storage::normalize_object_path;
use flow_like_types::{async_trait, json::json};

#[crate::register_node]
#[derive(Default)]
pub struct ParentNode {}

impl ParentNode {
    pub fn new() -> Self {
        ParentNode {}
    }
}

#[async_trait]
impl NodeLogic for ParentNode {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "parent",
            "Parent",
            "Gets the parent path from a path",
            "Data/Files/Path",
        );
        node.set_flowscript_name("path", "parent");
        node.set_receiver("path");
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
            "parent_path",
            "Parent Path",
            "Parent FlowPath",
            VariableType::Struct,
        )
        .set_schema::<FlowPath>();

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;
        let path: FlowPath = context.evaluate_pin("path").await?;

        let mut path = path.to_runtime(context).await?;
        let parent = path
            .path
            .as_ref()
            .rsplit_once('/')
            .map(|(parent, _)| parent)
            .unwrap_or("");
        path.path = normalize_object_path(parent);
        let path = path.serialize().await;

        context.set_pin_value("parent_path", json!(path)).await?;
        context.activate_exec_pin("exec_out").await?;
        Ok(())
    }
}
