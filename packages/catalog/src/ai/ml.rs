/// # Machine Learning Nodes
/// Collection of ML- and Deep Learning Nodes

/// ONNX Nodes
pub mod onnx;
pub mod tf_lite;
use flow_like::flow::node::NodeLogic;
use std::sync::Arc;

/// Add Machine Learning Nodes to Catalog Lib
pub async fn register_functions() -> Vec<Arc<dyn NodeLogic>> {
    let mut registry: Vec<Arc<dyn NodeLogic>> = Vec::new();
    registry.extend(onnx::register_functions().await);
    registry.extend(tf_lite::register_functions().await);
    registry
}
