pub mod embedding;
pub mod llm;

use crate::flow::node::NodeLogic;
use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn register_functions() -> Vec<Arc<dyn NodeLogic>> {
    let mut registry: Vec<Arc<dyn NodeLogic>> = Vec::new();

    let llm_registry = llm::register_functions().await;
    let embedding_registry = embedding::register_functions().await;

    registry.extend(llm_registry);
    registry.extend(embedding_registry);

    registry
}
