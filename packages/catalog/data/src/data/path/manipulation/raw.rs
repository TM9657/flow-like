use crate::data::path::FlowPath;
use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic},
    pin::PinOptions,
    variable::VariableType,
};
use flow_like_storage::display_object_path;
use flow_like_types::{async_trait, json::json};

#[crate::register_node]
#[derive(Default)]
pub struct RawPathNode {}

impl RawPathNode {
    pub fn new() -> Self {
        RawPathNode {}
    }
}

#[async_trait]
impl NodeLogic for RawPathNode {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "raw_path",
            "Raw Path",
            "Gets the human-readable path string",
            "Data/Files/Path",
        );
        node.set_flowscript_name("path", "rawPath");
        node.set_receiver("path");
        node.add_icon("/flow/icons/path.svg");

        node.add_input_pin("path", "Path", "FlowPath", VariableType::Struct)
            .set_schema::<FlowPath>()
            .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_output_pin(
            "raw_path",
            "Raw Path",
            "Human-readable path string with percent-encoding removed (e.g. 'Übersicht (2)#1.pdf')",
            VariableType::String,
        );

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let path: FlowPath = context.evaluate_pin("path").await?;

        let path = path.to_runtime(context).await?;
        let raw_path = display_object_path(&path.path);

        context.set_pin_value("raw_path", json!(raw_path)).await?;
        Ok(())
    }
}
