pub mod image;
pub mod text;

use crate::flow::node::NodeLogic;
use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn register_functions() -> Vec<Arc<Mutex<dyn NodeLogic>>> {
    let registry: Vec<Arc<Mutex<dyn NodeLogic>>> = Vec::new();

    registry
}
