use crate::data::path::FlowPath;
use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic},
    pin::PinOptions,
    variable::VariableType,
};
use flow_like_storage::display_file_name;
use flow_like_types::{async_trait, json::json};

#[crate::register_node]
#[derive(Default)]
pub struct ExtensionNode {}

impl ExtensionNode {
    pub fn new() -> Self {
        ExtensionNode {}
    }
}

#[async_trait]
impl NodeLogic for ExtensionNode {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "extension",
            "Extension",
            "Gets the file extension from a path",
            "Data/Files/Path",
        );
        node.set_flowscript_name("path", "extension");
        node.set_receiver("path");
        node.add_icon("/flow/icons/path.svg");

        node.add_input_pin("path", "Path", "FlowPath", VariableType::Struct)
            .set_schema::<FlowPath>()
            .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_output_pin(
            "extension",
            "Extension",
            "File extension with percent-encoding removed (e.g. 'doküment')",
            VariableType::String,
        );

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let path: FlowPath = context.evaluate_pin("path").await?;

        let extension = display_file_name(&path.object_path())
            .and_then(|name| {
                name.rsplit_once('.')
                    .map(|(_, extension)| extension.to_string())
            })
            .unwrap_or_default();

        context.set_pin_value("extension", json!(extension)).await?;
        Ok(())
    }
}
