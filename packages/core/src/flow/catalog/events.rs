pub mod simple_event;

use crate::flow::node::NodeLogic;
use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn register_functions() -> Vec<Arc<dyn NodeLogic>> {
    vec![Arc::new(simple_event::SimpleEventNode::default()) as Arc<dyn NodeLogic>]
}
