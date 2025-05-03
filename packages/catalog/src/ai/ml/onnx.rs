
pub mod classify;
pub mod detect;
pub mod feature;
pub mod load;

use flow_like::flow::node::NodeLogic;
use std::sync::Arc;

pub async fn register_functions() -> Vec<Arc<dyn NodeLogic>> {
    let nodes: Vec<Arc<dyn NodeLogic>> = vec![];
    nodes
}
