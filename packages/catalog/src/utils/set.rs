use flow_like::flow::node::NodeLogic;
use std::sync::Arc;

pub mod to_array;
pub mod push;
pub mod make;
pub mod clear;
pub mod discard;
pub mod has;

pub async fn register_functions() -> Vec<Arc<dyn NodeLogic>> {
    vec![
        Arc::new(to_array::SetToArrayNode::default()),
        Arc::new(clear::ClearSetNode::default()),
        Arc::new(discard::DiscardSetNode::default()),
        Arc::new(has::SetHasNode::default()),
        Arc::new(make::MakeSetNode::default()),
        Arc::new(push::PushSetNode::default()),
    ]
}
