pub mod generative;
pub mod processing;

use crate::flow::node::NodeLogic;
use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn register_functions() -> Vec<Arc<Mutex<dyn NodeLogic>>> {
    let mut registry: Vec<Arc<Mutex<dyn NodeLogic>>> = Vec::new();

    registry.extend(generative::register_functions().await);
    registry.extend(processing::register_functions().await);

    registry
}
