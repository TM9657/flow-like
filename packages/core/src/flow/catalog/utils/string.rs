use crate::flow::node::NodeLogic;
use std::sync::Arc;
use tokio::sync::Mutex;

pub mod compare;
pub mod contains;
pub mod ends_with;
pub mod join;
pub mod length;
pub mod replace;
pub mod similarity;
pub mod split;
pub mod starts_with;
pub mod to_lowercase;
pub mod to_uppercase;
pub mod trim;

pub async fn register_functions() -> Vec<Arc<Mutex<dyn NodeLogic>>> {
    let mut items: Vec<Arc<Mutex<dyn NodeLogic>>> = vec![
        Arc::new(Mutex::new(join::StringJoinNode::default())),
        Arc::new(Mutex::new(replace::StringReplaceNode::default())),
        Arc::new(Mutex::new(split::StringSplitNode::default())),
        Arc::new(Mutex::new(to_lowercase::StringToLowerNode::default())),
        Arc::new(Mutex::new(to_uppercase::StringToUpperNode::default())),
        Arc::new(Mutex::new(length::StringLengthNode::default())),
        Arc::new(Mutex::new(compare::CompareStringNode::default())),
        Arc::new(Mutex::new(trim::StringTrimNode::default())),
        Arc::new(Mutex::new(starts_with::StringStartsWithNode::default())),
        Arc::new(Mutex::new(ends_with::StringEndsWithNode::default())),
        Arc::new(Mutex::new(contains::StringContainsNode::default())),
    ];

    items.append(&mut similarity::register_functions().await);
    items
}
