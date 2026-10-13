use super::read_from_path::decode_image;
use flow_like::flow::{
    execution::{context::ExecutionContext, egress::GuardedHttpClient},
    node::{Node, NodeLogic},
    variable::VariableType,
};
use flow_like_catalog_core::NodeImage;
use flow_like_types::{Ok, async_trait, json::json};

#[crate::register_node]
#[derive(Default)]
pub struct ReadImageFromUrlNode {}

impl ReadImageFromUrlNode {
    pub fn new() -> Self {
        ReadImageFromUrlNode {}
    }
}

#[async_trait]
impl NodeLogic for ReadImageFromUrlNode {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "read_image_url",
            "Read Image (URL)",
            "Read image from path",
            "Image/Content",
        );
        node.set_flowscript_name("image", "readUrl");
        node.set_version(2);
        node.add_icon("/flow/icons/image.svg");

        node.add_input_pin(
            "exec_in",
            "Input",
            "Initiate Execution",
            VariableType::Execution,
        );
        node.add_input_pin(
            "signed_url",
            "Signed Url",
            "Signed Url",
            VariableType::String,
        );

        node.add_input_pin(
            "apply_exif_orientation",
            "Apply Exif Orientation",
            "Rotate and flip the image upright as its EXIF orientation tag says",
            VariableType::Boolean,
        )
        .set_default_value(Some(json!(true)));

        node.add_output_pin(
            "exec_out",
            "Output",
            "Done with the Execution",
            VariableType::Execution,
        );
        node.add_output_pin("image_out", "Image", "Image object", VariableType::Struct)
            .set_schema::<NodeImage>();

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;
        let signed_url: String = context.evaluate_pin("signed_url").await?;
        let bytes = GuardedHttpClient::new(context.execution_environment())?
            .get(&signed_url)?
            .send()
            .await
            .map_err(flow_like_types::reqwest::Error::without_url)?
            .bytes()
            .await
            .map_err(flow_like_types::reqwest::Error::without_url)?
            .to_vec();
        let apply_exif_orientation: bool = context.evaluate_pin("apply_exif_orientation").await?;
        let image = decode_image(bytes, apply_exif_orientation)?;

        let node_img = NodeImage::new(context, image).await;
        context.set_pin_value("image_out", json!(node_img)).await?;
        context.activate_exec_pin("exec_out").await?;
        Ok(())
    }
}
