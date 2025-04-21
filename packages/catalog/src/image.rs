
use flow_like::flow::execution::context::ExecutionContext;
use flow_like::flow::node::NodeLogic;
use flow_like_types::sync::Mutex;
use flow_like_types::{create_id, Cacheable, Result};
use flow_like_types::image::DynamicImage;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub mod content;
pub mod metadata;
pub mod transform;

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug)]
pub struct NodeImage {
    pub image_ref: String
}

pub struct NodeImageWrapper {
    pub image: Arc<Mutex<DynamicImage>>,
}

impl Cacheable for NodeImageWrapper {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl NodeImage {
    pub async fn new(ctx: &mut ExecutionContext, image: DynamicImage, ) -> Self {
        let id = create_id();
        let image_ref = Arc::new(Mutex::new(image));
        let wrapper = NodeImageWrapper {
            image: image_ref.clone(),
        };
        ctx.cache.write().await.insert(id.clone(), Arc::new(wrapper));
        NodeImage { image_ref: id }
    }

    pub async fn copy_image(&self, ctx: &mut ExecutionContext) -> Result<Self> {
        let image = ctx.cache.read().await.get(&self.image_ref).cloned().ok_or_else(|| {
            flow_like_types::anyhow!("Image not found in cache")
        })?;
        let image_wrapper = image.as_any().downcast_ref::<NodeImageWrapper>().ok_or_else(|| {
            flow_like_types::anyhow!("Could not downcast to NodeImageWrapper")
        })?;
        let image = image_wrapper.image.lock().await.clone();
        let new_id = create_id();
        let new_image_ref = Arc::new(Mutex::new(image.clone()));
        let new_wrapper = NodeImageWrapper {
            image: new_image_ref.clone(),
        };
        ctx.cache.write().await.insert(new_id.clone(), Arc::new(new_wrapper));
        let new_image = NodeImage { image_ref: new_id };
        Ok(new_image)
    }

    pub async fn get_image(&self, ctx: &mut ExecutionContext) -> Result<Arc<Mutex<DynamicImage>>> {
        let image = ctx.cache.read().await.get(&self.image_ref).cloned().ok_or_else(|| {
            flow_like_types::anyhow!("Image not found in cache")
        })?;
        let image_wrapper = image.as_any().downcast_ref::<NodeImageWrapper>().ok_or_else(|| {
            flow_like_types::anyhow!("Could not downcast to NodeImageWrapper")
        })?;
        let image = image_wrapper.image.clone();
        Ok(image)
    }
}

pub async fn register_functions() -> Vec<Arc<dyn NodeLogic>> {
    let nodes: Vec<Arc<dyn NodeLogic>> = vec![
        Arc::new(metadata::dims::ImageDimsNode::default()),
        Arc::new(content::read_from_path::ReadImagePathNode::default()),
        Arc::new(content::read_from_url::ReadImageFromUrlNode::default()),
        Arc::new(content::write_to_path::WriteImageNode::default()),
        Arc::new(transform::resize::ResizeImageNode::default()),
    ];
    nodes
}