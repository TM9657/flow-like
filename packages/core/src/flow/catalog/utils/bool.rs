use crate::flow::node::NodeLogic;
use std::sync::Arc;

pub mod and;
pub mod equal;
pub mod not;
pub mod or;
pub mod random;
pub mod xor;

pub async fn register_functions() -> Vec<Arc<dyn NodeLogic>> {
    vec![
        Arc::new(and::BoolAnd::default()),
        Arc::new(or::BoolOr::default()),
        Arc::new(equal::BoolEqual::default()),
        Arc::new(not::BoolNot::default()),
        Arc::new(xor::BoolXor::default()),
        Arc::new(random::RandomBoolNode::default()),
    ]
}
