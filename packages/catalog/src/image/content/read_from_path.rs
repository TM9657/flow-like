
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
use flow_like_types::{async_trait, image::{DynamicImage, ImageDecoder, ImageFormat, ImageReader}, json::json, Ok};
use std::io::Cursor;


#[derive(Default)]
pub struct ReadImagePathNode {}

impl ReadImagePathNode {
    pub fn new() -> Self {
        ReadImagePathNode {}
    }
}

#[async_trait]
impl NodeLogic for ReadImagePathNode { 
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "read_image",
            "Read Image",
            "Read image from path",
            "Image/Content",
        );
        node.add_icon("/flow/icons/dir.svg");
        
        // inputs
        node.add_input_pin(
            "exec_in",
            "Input",
            "Initiate Execution",
            VariableType::Execution,
        );
        node.add_input_pin(
            "path",
            "Path",
            "FlowPath",
            VariableType::Struct,
        )
            .set_schema::<FlowPath>()
            .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_input_pin(
            "apply_exif", 
            "Apply Exif", 
            "Apply Exif Orientation", 
            VariableType::Boolean,
        )
            .set_default_value(Some(json!(false)));

        // outputs
        node.add_output_pin(
            "exec_out",
            "Output",
            "Done with the Execution",
            VariableType::Execution,
        );
        node.add_output_pin(
            "image_out",
            "Image",
            "Image object",
            VariableType::Struct,
        )
            .set_schema::<NodeImage>();
        
        node 
        
    }
    
    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        // get inputs
        context.deactivate_exec_pin("exec_out").await?;
        let path: FlowPath = context.evaluate_pin("path").await?;
        let path_runtime = path.to_runtime(context).await?;
        let bytes = path_runtime.store
            .as_generic()
            .get(&path_runtime.path).await?
            .bytes().await?
            .to_vec();
        let apply_exif: bool = context.evaluate_pin("apply_exif").await?;
        
        // load image
        let encoded = if apply_exif {
            // decode-encode image to apply exif-orientation
            let mut tmp_bytes_out: Vec<u8> = Vec::new();
            let reader = ImageReader::new(Cursor::new(bytes)).with_guessed_format()?;
            let mut decoder = reader.into_decoder()?;
            let orientation = decoder.orientation()?;
            let mut img = DynamicImage::from_decoder(decoder)?;
            img.apply_orientation(orientation);  // compensate potential rotations from metadata
            img.write_to(&mut Cursor::new(&mut tmp_bytes_out), ImageFormat::Png)?;  // encode & write to bytes (might be expensive)
            tmp_bytes_out
        } else {
            // pass-on encoded as no transform required
            bytes
        };

        // set outputs
        let node_img = NodeImage::from_encoded(encoded)?;
        context.set_pin_value("image_out", json!(node_img)).await?;
        context.activate_exec_pin("exec_out").await?;
        Ok(())
    }
}