use flow_like::flow::node::NodeLogic;
use std::sync::Arc;

pub mod clear;
pub mod difference;
pub mod discard;
pub mod from_array;
pub mod has;
pub mod insert;
pub mod is_empty;
pub mod is_subset;
pub mod is_superset;
pub mod make;
pub mod mutual;
pub mod pop;
pub mod size;
pub mod to_array;
pub mod union;

pub async fn register_functions() -> Vec<Arc<dyn NodeLogic>> {
    vec![
        Arc::new(to_array::SetToArrayNode::default()),
        Arc::new(clear::ClearSetNode::default()),
        Arc::new(from_array::ArrayToSetNode::default()),
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
        Arc::new(mutual::IsMutualSetNode::default()),
    ]
}
