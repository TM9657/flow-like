
use crate::image::NodeImage;
use flow_like::{
    flow::{
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        pin::PinOptions,
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_types::{async_trait, image::codecs::jpeg::JpegEncoder, json::json, Ok};
use std::io::Cursor;

#[derive(Default)]
pub struct AsJpgNode {}

impl AsJpgNode {
    pub fn new() -> Self {
        AsJpgNode{}
    }
}

#[async_trait]
impl NodeLogic for AsJpgNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "as_jpg",
            "Encode as JPG",
            "Encode Image as JPEG with Quality",
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
            "image_in",
            "Image",
            "Image object",
            VariableType::Struct,
        )
            .set_schema::<NodeImage>()
            .set_options(PinOptions::new()
                .set_enforce_schema(true)
                .build()
            );

        node.add_input_pin(
            "quality", 
            "Quality", 
            "JPEG Encoding Quality", 
            VariableType::Integer,
        )
            .set_options(PinOptions::new()
                .set_enforce_schema(true)
                .set_range((0., 100.))
                .build()
            )
            .set_default_value(Some(json!(75))
        );

        // outputs
        // todo: output as tuple
        node.add_output_pin(
            "exec_out",
            "Output",
            "Done with the Execution",
            VariableType::Execution,
        );
        node.add_output_pin(
            "image_out",
            "Image",
            "JPEG-Encoded Output Image",
            VariableType::Struct,
        )
            .set_schema::<NodeImage>();

        node

    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        // get inputs
        context.deactivate_exec_pin("exec_out").await?;
        let node_image: NodeImage = context.evaluate_pin("image_in").await?;
        let quality: f64 = context.evaluate_pin("quality").await?;
        let (img, _format) = node_image.as_decoded_with_format()?;

        // custom jpeg encoded to set quality
        let mut encoded: Vec<u8> = Vec::new();
        let cursor = Cursor::new(&mut encoded);
        let encoder = JpegEncoder::new_with_quality(cursor, quality as u8);
        img.write_with_encoder(encoder)?;

        // set outputs
        let img_out = NodeImage::from_encoded(encoded)?;
        context.set_pin_value("image_out", json!(img_out)).await?;
        context.activate_exec_pin("exec_out").await?;
        Ok(())
    }
}