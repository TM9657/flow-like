pub mod get;
pub mod set;

use crate::flow::node::NodeLogic;
use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn register_functions() -> Vec<Arc<dyn NodeLogic>> {
    let mut registry: Vec<Arc<dyn NodeLogic>> = Vec::new();
    registry.push(Arc::new(get::GetVariable::default()));
    registry.push(Arc::new(set::SetVariable::default()));
    registry
}
