pub mod try_transform;

use crate::flow::node::NodeLogic;
use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn register_functions() -> Vec<Arc<Mutex<dyn NodeLogic>>> {
    vec![Arc::new(Mutex::new(
        try_transform::TryTransformNode::default(),
    ))]
}
