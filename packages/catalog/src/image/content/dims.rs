
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
use flow_like_types::{async_trait, json::json, Ok, image::GenericImageView};

#[derive(Default)]
pub struct ImageDimsNode {}

impl ImageDimsNode {
    pub fn new() -> Self {
        ImageDimsNode{}
    }
}

#[async_trait]
impl NodeLogic for ImageDimsNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "get_dimensions",
            "Get Dimensions",
            "Get Image Dimensions",
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
            .set_options(PinOptions::new().set_enforce_schema(true).build());

        // outputs
        // todo: output as tuple
        node.add_output_pin(
            "exec_out",
            "Output",
            "Done with the Execution",
            VariableType::Execution,
        );
        node.add_output_pin(
            "width",
            "width",
            "Image Width",
            VariableType::Integer,
        );
        node.add_output_pin(
            "height",
            "height",
            "Image Height",
            VariableType::Integer,
        );
        node

    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        // get inputs
        context.deactivate_exec_pin("exec_out").await?;
        let node_image: NodeImage = context.evaluate_pin("image_in").await?;

        // get dimensions
        let (img, _format) = node_image.as_decoded_with_format()?;
        let (width, height) = img.dimensions();

        // set outputs
        context.set_pin_value("width", json!(width)).await?;
        context.set_pin_value("height", json!(height)).await?;
        context.activate_exec_pin("exec_out").await?;
        Ok(())
    }
}