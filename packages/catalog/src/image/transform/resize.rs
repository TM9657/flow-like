
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
use flow_like_types::{async_trait, image::{self, imageops::FilterType, GenericImageView}, json::json, Ok};

#[derive(Default)]
pub struct ResizeImageNode {}

impl ResizeImageNode {
    pub fn new() -> Self {
        ResizeImageNode {}
    }
}

#[async_trait]
impl NodeLogic for ResizeImageNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "resize_image",
            "Resize Image",
            "Resize Image",
            "Image/Transform",
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

        node.add_input_pin(
            "width",
            "Target Width",
            "Resized Image Target Width",
            VariableType::Integer,
        );
        node.add_input_pin(
            "height",
            "Target Height",
            "Resized Image Target Height",
            VariableType::Integer,
        );

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

        node.add_output_pin(
            "result_width",
            "Result Width",
            "Resized Image Result Width",
            VariableType::Integer,
        );
        node.add_output_pin(
            "result_height",
            "Result Height",
            "Resized Image Result Height",
            VariableType::Integer,
        );
        node

    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;

        // get inputs
        let node_img: NodeImage = context.evaluate_pin("image_in").await?;
        let target_width: u32 = context.evaluate_pin("width").await?;
        let target_height: u32 = context.evaluate_pin("height").await?;
        // todo: allow resize_exact and resize_to_fill via enum
        // todo: allow filters via FilterType

        // get image
        let img = node_img.get_image(context).await?;

        // resize image
        let resized_img = img.resize(target_width, target_height, FilterType::Lanczos3);
        let (result_width, result_height) = resized_img.dimensions();        
        let resized_node_img = NodeImage::from_bytes(resized_img.as_bytes().to_vec())?;

        // set outputs
        context.set_pin_value("image_out", json!(resized_node_img)).await?;
        context.set_pin_value("result_width", json!(result_width)).await?;
        context.set_pin_value("result_height", json!(result_height)).await?;
        context.activate_exec_pin("exec_out").await?;
        Ok(())
    }
}