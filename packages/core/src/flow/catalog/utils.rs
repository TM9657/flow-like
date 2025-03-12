pub mod array;
pub mod bool;
pub mod cuid;
pub mod env;
pub mod float;
pub mod int;
pub mod math;
pub mod string;
pub mod types;
pub mod vector;
pub mod json;

use crate::flow::node::NodeLogic;
use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn register_functions() -> Vec<Arc<Mutex<dyn NodeLogic>>> {
    let mut registry: Vec<Arc<Mutex<dyn NodeLogic>>> = Vec::new();
    registry.push(Arc::new(Mutex::new(cuid::CuidNode::default())));
    registry.append(&mut types::register_functions().await);
    registry.append(&mut bool::register_functions().await);
    registry.append(&mut env::register_functions().await);
    registry.append(&mut string::register_functions().await);
    registry.append(&mut array::register_functions().await);
    registry.append(&mut vector::register_functions().await);
    registry.append(&mut float::register_functions().await);
    registry.append(&mut int::register_functions().await);
    registry.push(Arc::new(Mutex::new(math::eval::EvalNode::default())));
    registry
}
