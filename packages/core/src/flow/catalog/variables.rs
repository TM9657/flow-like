pub mod get;
pub mod set;

use crate::flow::node::NodeLogic;
use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn register_functions() -> Vec<Arc<Mutex<dyn NodeLogic>>> {
    let mut registry: Vec<Arc<Mutex<dyn NodeLogic>>> = Vec::new();
    registry.push(Arc::new(Mutex::new(get::GetVariable::default())));
    registry.push(Arc::new(Mutex::new(set::SetVariable::default())));
    registry
}
