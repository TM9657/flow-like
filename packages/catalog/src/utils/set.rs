use flow_like::flow::node::NodeLogic;
use std::sync::Arc;

pub mod to_array;
pub mod insert;
pub mod make;
pub mod clear;
pub mod discard;
pub mod has;
pub mod is_empty;
pub mod size;
pub mod union;
pub mod difference;
pub mod is_subset;
pub mod is_superset;

pub async fn register_functions() -> Vec<Arc<dyn NodeLogic>> {
    vec![
        Arc::new(to_array::SetToArrayNode::default()),
        Arc::new(clear::ClearSetNode::default()),
        Arc::new(discard::DiscardSetNode::default()),
        Arc::new(has::SetHasNode::default()),
        Arc::new(make::MakeSetNode::default()),
        Arc::new(insert::InsertSetNode::default()),
    ]
}
