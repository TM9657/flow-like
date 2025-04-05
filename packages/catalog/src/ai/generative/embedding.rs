use flow_like::{bit::BitTypes, flow::node::NodeLogic};
use flow_like_model_provider::{
    embedding::EmbeddingModelLogic, image_embedding::ImageEmbeddingModelLogic,
};
use flow_like_types::{
    Cacheable, JsonSchema,
    json::{Deserialize, Serialize},
};
use load::LoadModelNode;
use std::{any::Any, sync::Arc};

pub mod image;
pub mod load;
pub mod text;

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct CachedEmbeddingModel {
    pub cache_key: String,
    pub model_type: BitTypes,
}

pub struct CachedEmbeddingModelObject {
    pub text_model: Option<Arc<dyn EmbeddingModelLogic>>,
    pub image_model: Option<Arc<dyn ImageEmbeddingModelLogic>>,
}

impl Cacheable for CachedEmbeddingModelObject {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

pub async fn register_functions() -> Vec<Arc<dyn NodeLogic>> {
    let mut nodes: Vec<Arc<dyn NodeLogic>> = vec![Arc::new(LoadModelNode::default())];
    nodes.extend(text::register_functions().await);
    nodes.extend(image::register_functions().await);
    nodes
}
