pub mod bool;
pub mod float;
pub mod get;
pub mod int;
pub mod set;
pub mod string;

use crate::flow::node::NodeLogic;
use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn register_functions() -> Vec<Arc<Mutex<dyn NodeLogic>>> {
    let mut registry: Vec<Arc<Mutex<dyn NodeLogic>>> = Vec::new();
    registry.append(&mut bool::register_functions().await);
    registry.push(Arc::new(Mutex::new(get::GetVariable::default())));
    registry.push(Arc::new(Mutex::new(set::SetVariable::default())));
    registry
}
