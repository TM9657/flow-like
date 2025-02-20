pub mod cuid;
pub mod types;

use crate::flow::node::NodeLogic;
use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn register_functions() -> Vec<Arc<Mutex<dyn NodeLogic>>> {
    let mut registry: Vec<Arc<Mutex<dyn NodeLogic>>> = Vec::new();
    registry.push(Arc::new(Mutex::new(cuid::CuidNode::default())));
    registry.append(&mut types::register_functions().await);
    registry
}
