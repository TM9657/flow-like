use flow_like::flow::node::NodeLogic;
use std::sync::Arc;
pub mod kmeans;

pub async fn register_functions() -> Vec<Arc<dyn NodeLogic>> {
    let nodes: Vec<Arc<dyn NodeLogic>> = vec![
        Arc::new(kmeans::FitKMeansNode::default())
    ];
    nodes
}
