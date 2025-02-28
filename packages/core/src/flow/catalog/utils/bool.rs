use crate::flow::node::NodeLogic;
use std::sync::Arc;
use tokio::sync::Mutex;

pub mod and;
pub mod equal;
pub mod not;
pub mod or;
pub mod random;
pub mod xor;

pub async fn register_functions() -> Vec<Arc<Mutex<dyn NodeLogic>>> {
    vec![
        Arc::new(Mutex::new(and::BoolAnd::default())),
        Arc::new(Mutex::new(or::BoolOr::default())),
        Arc::new(Mutex::new(equal::BoolEqual::default())),
        Arc::new(Mutex::new(not::BoolNot::default())),
        Arc::new(Mutex::new(xor::BoolXor::default())),
        Arc::new(Mutex::new(random::RandomBoolNode::default())),
    ]
}
