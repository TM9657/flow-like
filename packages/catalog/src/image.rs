
use flow_like::flow::execution::context::ExecutionContext;
use flow_like::flow::node::NodeLogic;
use flow_like_types::Result;
use flow_like_types::image::{DynamicImage, ImageReader};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::io::Cursor;

pub mod content;
pub mod transform;

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug)]
pub struct NodeImage {
    pub image_bytes: Vec<u8>
}

impl NodeImage {

    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self> {
        Ok(Self {
            image_bytes: bytes,
        })
    }

    async fn get_image(&self, context: &mut ExecutionContext) -> Result<DynamicImage> {
        let dynamic_image = ImageReader::new(Cursor::new(&self.image_bytes))
            .with_guessed_format()?
            .decode()?;  // decode image (might be expensive?)
        Ok(dynamic_image)
    }
}

pub async fn register_functions() -> Vec<Arc<dyn NodeLogic>> {
    let nodes: Vec<Arc<dyn NodeLogic>> = vec![
        Arc::new(content::read_from_path::ReadImagePathNode::default()),
        Arc::new(content::dims::ImageDimsNode::default()),
        Arc::new(transform::resize::ResizeImageNode::default()),
    ];
    nodes
}