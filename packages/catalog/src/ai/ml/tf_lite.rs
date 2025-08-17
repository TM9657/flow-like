/// # TFLite Nodes
/// Loading and running TensorFlow Lite models

use flow_like::flow::node::NodeLogic;
use std::sync::Arc;

pub mod classify;

/// Register TFLite-related nodes
pub async fn register_functions() -> Vec<Arc<dyn NodeLogic>> {
    vec![Arc::new(classify::TfliteImageClassificationNode::default())]
}