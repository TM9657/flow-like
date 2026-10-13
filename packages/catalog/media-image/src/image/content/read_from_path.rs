use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic},
    pin::PinOptions,
    variable::VariableType,
};
use flow_like_catalog_core::{FlowPath, NodeImage};
use flow_like_types::{
    Ok, async_trait,
    image::{DynamicImage, ImageDecoder, ImageReader},
    json::json,
};
use std::io::Cursor;

/// Decodes an image and, when asked, turns it upright as its EXIF orientation tag says.
pub(crate) fn decode_image(
    bytes: Vec<u8>,
    apply_exif_orientation: bool,
) -> flow_like_types::Result<DynamicImage> {
    let reader = ImageReader::new(Cursor::new(bytes)).with_guessed_format()?;
    let mut decoder = reader.into_decoder()?;
    let orientation = decoder.orientation()?;
    let mut image = DynamicImage::from_decoder(decoder)?;
    if apply_exif_orientation {
        image.apply_orientation(orientation);
    }
    Ok(image)
}

#[crate::register_node]
#[derive(Default)]
pub struct ReadImagePathNode {}

impl ReadImagePathNode {
    pub fn new() -> Self {
        ReadImagePathNode {}
    }
}

#[async_trait]
impl NodeLogic for ReadImagePathNode {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "read_image",
            "Read Image",
            "Read image from path",
            "Image/Content",
        );
        node.set_flowscript_name("image", "read");
        node.set_version(2);
        node.add_icon("/flow/icons/image.svg");

        node.add_input_pin(
            "exec_in",
            "Input",
            "Initiate Execution",
            VariableType::Execution,
        );
        node.add_input_pin("path", "Path", "FlowPath", VariableType::Struct)
            .set_schema::<FlowPath>()
            .set_options(PinOptions::new().set_enforce_schema(true).build());

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
        let path: FlowPath = context.evaluate_pin("path").await?;
        let bytes = path.get(context, false).await?;
        let apply_exif_orientation: bool = context.evaluate_pin("apply_exif_orientation").await?;
        let image = decode_image(bytes, apply_exif_orientation)?;

        let node_img = NodeImage::new(context, image).await;
        context.set_pin_value("image_out", json!(node_img)).await?;
        context.activate_exec_pin("exec_out").await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{ReadImagePathNode, decode_image};
    use crate::image::content::read_from_url::ReadImageFromUrlNode;
    use flow_like::flow::node::NodeLogic;
    use flow_like_types::image::{
        ExtendedColorType, GenericImageView, ImageEncoder, Rgb, RgbImage, codecs::jpeg::JpegEncoder,
    };

    /// A little-endian TIFF block whose only entry is Orientation 6: the stored frame
    /// has to be turned 90 degrees clockwise to be upright.
    const ROTATE_90_EXIF: [u8; 26] = [
        0x49, 0x49, 0x2a, 0x00, 0x08, 0x00, 0x00, 0x00, 0x01, 0x00, 0x12, 0x01, 0x03, 0x00, 0x01,
        0x00, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ];

    fn landscape_frame_of_a_portrait_photo() -> Vec<u8> {
        let frame = RgbImage::from_pixel(32, 16, Rgb([200, 40, 40]));
        let mut bytes = Vec::new();
        let mut encoder = JpegEncoder::new(&mut bytes);
        encoder.set_exif_metadata(ROTATE_90_EXIF.to_vec()).unwrap();
        encoder
            .write_image(frame.as_raw(), 32, 16, ExtendedColorType::Rgb8)
            .unwrap();
        bytes
    }

    #[test]
    fn exif_orientation_is_applied_only_when_requested() {
        let bytes = landscape_frame_of_a_portrait_photo();
        let upright = decode_image(bytes.clone(), true).unwrap();
        assert_eq!(upright.dimensions(), (16, 32));
        let stored = decode_image(bytes, false).unwrap();
        assert_eq!(stored.dimensions(), (32, 16));
    }

    #[test]
    fn read_image_nodes_apply_exif_orientation_by_default() {
        for node in [
            ReadImagePathNode::new().get_node(),
            ReadImageFromUrlNode::new().get_node(),
        ] {
            assert_eq!(node.version, Some(2), "{}", node.name);
            assert!(
                node.get_pin_by_name("apply_exif").is_none(),
                "{}",
                node.name
            );
            let pin = node
                .get_pin_by_name("apply_exif_orientation")
                .expect("orientation pin");
            let default: bool =
                flow_like_types::json::from_slice(pin.default_value.as_ref().unwrap()).unwrap();
            assert!(default, "{}", node.name);
        }
    }
}
