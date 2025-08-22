pub mod agent;
pub mod embedding;
pub mod llm;

use flow_like::flow::node::NodeLogic;
use std::sync::Arc;

pub async fn register_functions() -> Vec<Arc<dyn NodeLogic>> {
    let mut registry: Vec<Arc<dyn NodeLogic>> = Vec::new();

    let llm_registry = llm::register_functions().await;
    let embedding_registry = embedding::register_functions().await;
    let agent_registry = agent::register_functions().await;

    registry.extend(llm_registry);
    registry.extend(embedding_registry);
    registry.extend(agent_registry);

    registry
}
