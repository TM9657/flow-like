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
pub mod pop;
pub mod mutual;

pub async fn register_functions() -> Vec<Arc<dyn NodeLogic>> {
    vec![
        Arc::new(to_array::SetToArrayNode::default()),
        Arc::new(clear::ClearSetNode::default()),
        Arc::new(discard::DiscardSetNode::default()),
        Arc::new(has::SetHasNode::default()),
        Arc::new(make::MakeSetNode::default()),
        Arc::new(insert::InsertSetNode::default()),
        Arc::new(union::UnionSetNode::default()),
        Arc::new(difference::DifferenceSetNode::default()),
        Arc::new(size::SetGetSizeNode::default()),
        Arc::new(pop::PopSetNode::default()),
        Arc::new(is_superset::SetIsSuperSetNode::default()),
        Arc::new(is_subset::SetIsSubsetNode::default()),
        Arc::new(is_empty::SetIsEmptyNode::default()),
        Arc::new(mutual::IsMutualSetNode::default()),
    ]
}
