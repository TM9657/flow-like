pub mod generative;
pub mod onnx;
pub mod ml;
pub mod processing;
pub mod teachable_machine;
use flow_like::flow::node::NodeLogic;
use std::sync::Arc;

pub async fn register_functions() -> Vec<Arc<dyn NodeLogic>> {
    let mut registry: Vec<Arc<dyn NodeLogic>> = Vec::new();
    registry.extend(onnx::register_functions().await);
    registry.extend(teachable_machine::register_functions().await);
    registry.extend(generative::register_functions().await);
    registry.extend(processing::register_functions().await);
    registry.extend(ml::register_functions().await);
    registry
}
