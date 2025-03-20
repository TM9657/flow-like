use crate::flow::node::NodeLogic;
use std::sync::Arc;
use tokio::sync::Mutex;

pub mod contains;
pub mod ends_with;
pub mod equal;
pub mod format;
pub mod join;
pub mod length;
pub mod replace;
pub mod similarity;
pub mod split;
pub mod starts_with;
pub mod to_lowercase;
pub mod to_uppercase;
pub mod trim;
pub mod unequal;

pub async fn register_functions() -> Vec<Arc<dyn NodeLogic>> {
    let mut items: Vec<Arc<dyn NodeLogic>> = vec![
        Arc::new(format::FormatStringNode::default()),
        Arc::new(join::StringJoinNode::default()),
        Arc::new(replace::StringReplaceNode::default()),
        Arc::new(split::StringSplitNode::default()),
        Arc::new(to_lowercase::StringToLowerNode::default()),
        Arc::new(to_uppercase::StringToUpperNode::default()),
        Arc::new(length::StringLengthNode::default()),
        Arc::new(equal::EqualStringNode::default()),
        Arc::new(unequal::UnEqualStringNode::default()),
        Arc::new(trim::StringTrimNode::default()),
        Arc::new(starts_with::StringStartsWithNode::default()),
        Arc::new(ends_with::StringEndsWithNode::default()),
        Arc::new(contains::StringContainsNode::default()),
    ];

    items.append(&mut similarity::register_functions().await);
    items
}
