pub mod get;
pub mod set;

use flow_like::flow::node::NodeLogic;
use std::sync::Arc;

pub async fn register_functions() -> Vec<Arc<dyn NodeLogic>> {
    let mut registry: Vec<Arc<dyn NodeLogic>> = Vec::new();
    registry.push(Arc::new(get::GetVariable::default()));
    registry.push(Arc::new(set::SetVariable::default()));
    registry
}
