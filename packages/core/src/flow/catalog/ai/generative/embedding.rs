pub mod image;
pub mod load;
pub mod text;

use crate::{bit::BitTypes, flow::node::NodeLogic};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct CachedEmbeddingModel {
    pub cache_key: String,
    pub model_type: BitTypes,
}

pub async fn register_functions() -> Vec<Arc<Mutex<dyn NodeLogic>>> {
    vec![Arc::new(Mutex::new(load::LoadModelNode::default()))]
}
