
use crate::{image::NodeImage, storage::path::FlowPath};
use flow_like::{
    flow::{
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        pin::PinOptions,
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_storage::object_store::PutPayload;
use flow_like_types::{async_trait, Bytes};
use std::io::Cursor;

#[derive(Default)]
pub struct WriteImageNode {}

impl WriteImageNode {
    pub fn new() -> Self {
        WriteImageNode {}
    }
}

#[async_trait]
impl NodeLogic for WriteImageNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "write_image",
            "Write Image",
            "Write image to path",
            "Image/Content",
        );
        node.add_icon("/flow/icons/dir.svg"); // Consider a more appropriate icon

        // inputs
        node.add_input_pin(
            "exec_in",
            "Input",
            "Initiate Execution",
            VariableType::Execution,
        );

        node.add_input_pin(
            "image_in",
            "Image",
            "The image to write to path",
            VariableType::Struct,
        )
            .set_schema::<NodeImage>()
            .set_options(PinOptions::new()
                .set_enforce_schema(true)
                .build()
        );

        node.add_input_pin(
            "path", 
            "Path", 
            "FlowPath", 
            VariableType::Struct
        )
            .set_schema::<FlowPath>()
            .set_options(PinOptions::new()
                .set_enforce_schema(true)
                .build()
        );

        // outputs
        node.add_output_pin(
            "exec_out",
            "Output",
            "Done with the Execution",
            VariableType::Execution,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;

        // get inputs
        let path: FlowPath = context.evaluate_pin("path").await?;
        let path = path.to_runtime(context).await?;
        let node_image: NodeImage = context.evaluate_pin("image_in").await?;

        // encode image based on path extension
        let (img, format) = node_image.as_decoded_with_format()?;
        let mut bytes_out: Vec<u8> = Vec::new();
        img.write_to(&mut Cursor::new(&mut bytes_out), format)?;

        // write image to path
        let store = path.store.as_generic();
        let payload = PutPayload::from_bytes(Bytes::from(bytes_out));
        store.put(&path.path, payload).await?;

        // set outputs
        context.activate_exec_pin("exec_out").await?;
        Ok(())
    }
}
