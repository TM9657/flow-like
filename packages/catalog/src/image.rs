
use flow_like::flow::node::NodeLogic;
use flow_like_types::Result;
use flow_like_types::image::{DynamicImage, ImageFormat, ImageReader};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::io::Cursor;

pub mod content;
pub mod transform;

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug)]
pub struct NodeImage {
    pub encoded: Vec<u8>
}

impl NodeImage {
    /// construct NodeImage from already encoded image - make sure to have used correct format
    pub fn from_encoded(encoded: Vec<u8>) -> Result<Self> {
        Ok(Self { encoded })
    }

    /// construct NodeImage from image::DynamicImage to vec with correct format
    pub fn from_decoded(decoded: &DynamicImage, format: ImageFormat) -> Result<Self> {
        let mut encoded: Vec<u8> = Vec::new();
        decoded.write_to(&mut Cursor::new(&mut encoded), format)?;
        Ok(Self { encoded: encoded })
    }

    /// retrieve as decoded image::DynamicImage + guessed format for downstream re-encoding
    pub fn as_decoded_with_format(&self) -> Result<(DynamicImage, ImageFormat)> {
        // todo: fallback to format from extension when guessing fails
        let reader = ImageReader::new(Cursor::new(&self.encoded)).with_guessed_format()?;
        let guessed_format = reader.format().expect("with_guessed_format always sets format");
        let img = reader.decode()?;
        Ok((img, guessed_format))
    }
}

pub async fn register_functions() -> Vec<Arc<dyn NodeLogic>> {
    let nodes: Vec<Arc<dyn NodeLogic>> = vec![
        Arc::new(content::as_jpg::AsJpgNode::default()),
        Arc::new(content::dims::ImageDimsNode::default()),
        Arc::new(content::read_from_path::ReadImagePathNode::default()),
        Arc::new(content::write_to_path::WriteImageNode::default()),
        Arc::new(transform::resize::ResizeImageNode::default()),
    ];
    nodes
}