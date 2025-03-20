use crate::flow::node::NodeLogic;
use std::sync::Arc;
use tokio::sync::Mutex;

pub mod clear;
pub mod find_item;
pub mod get;
pub mod includes;
pub mod len;
pub mod make;
pub mod pop;
pub mod push;
pub mod remove_index;
pub mod set;

pub async fn register_functions() -> Vec<Arc<dyn NodeLogic>> {
    vec![
        Arc::new(len::ArrayLengthNode::default()),
        Arc::new(get::GetArrayElementNode::default()),
        Arc::new(includes::ArrayIncludesNode::default()),
        Arc::new(make::MakeArrayNode::default()),
        Arc::new(pop::PopArrayNode::default()),
        Arc::new(push::PushArrayNode::default()),
        Arc::new(set::SetIndexArrayNode::default()),
        Arc::new(remove_index::RemoveArrayIndexNode::default()),
        Arc::new(clear::ClearArrayNode::default()),
        Arc::new(find_item::FindItemInArrayNode::default()),
    ]
}
