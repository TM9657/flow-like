/// # Machine Learning Nodes
pub mod classification;
pub mod clustering;
pub mod dataset;
pub mod reduction;
use flow_like::flow::node::NodeLogic;
use std::sync::Arc;

/// Add Machine Learning Nodes to Catalog Lib
pub async fn register_functions() -> Vec<Arc<dyn NodeLogic>> {
    let mut registry: Vec<Arc<dyn NodeLogic>> = Vec::new();
    registry.extend(clustering::register_functions().await);
    registry
}
