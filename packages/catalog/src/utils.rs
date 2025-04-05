pub mod array;
pub mod bool;
pub mod cuid;
pub mod env;
pub mod float;
pub mod int;
pub mod json;
pub mod math;
pub mod string;
pub mod types;
pub mod vector;

use flow_like::flow::node::NodeLogic;
use std::sync::Arc;

pub async fn register_functions() -> Vec<Arc<dyn NodeLogic>> {
    let mut registry: Vec<Arc<dyn NodeLogic>> = Vec::new();
    registry.push(Arc::new(cuid::CuidNode::default()));
    registry.push(Arc::new(json::repair_parse::RepairParseNode::default()));
    registry.append(&mut types::register_functions().await);
    registry.append(&mut bool::register_functions().await);
    registry.append(&mut env::register_functions().await);
    registry.append(&mut string::register_functions().await);
    registry.append(&mut array::register_functions().await);
    registry.append(&mut vector::register_functions().await);
    registry.append(&mut float::register_functions().await);
    registry.append(&mut int::register_functions().await);
    registry.push(Arc::new(math::eval::EvalNode::default()));
    registry
}
