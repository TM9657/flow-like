
pub mod onnx;

use flow_like::flow::node::NodeLogic;
use std::sync::Arc;

pub async fn register_functions() -> Vec<Arc<dyn NodeLogic>> {
    let mut registry: Vec<Arc<dyn NodeLogic>> = Vec::new();
    registry.extend(onnx::register_functions().await);
    registry
}