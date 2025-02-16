pub mod generative;

use crate::flow::node::NodeLogic;
use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn register_functions() -> Vec<Arc<Mutex<dyn NodeLogic>>> {
    let mut registry: Vec<Arc<Mutex<dyn NodeLogic>>> = Vec::new();

    let generative_registry = generative::register_functions().await;
    registry.extend(generative_registry);

    registry
}
