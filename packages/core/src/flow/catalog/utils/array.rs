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

pub async fn register_functions() -> Vec<Arc<Mutex<dyn NodeLogic>>> {
    vec![
        Arc::new(Mutex::new(len::ArrayLengthNode::default())),
        Arc::new(Mutex::new(get::GetArrayElementNode::default())),
        Arc::new(Mutex::new(includes::ArrayIncludesNode::default())),
        Arc::new(Mutex::new(make::MakeArrayNode::default())),
        Arc::new(Mutex::new(pop::PopArrayNode::default())),
        Arc::new(Mutex::new(push::PushArrayNode::default())),
        Arc::new(Mutex::new(set::SetIndexArrayNode::default())),
        Arc::new(Mutex::new(remove_index::RemoveArrayIndexNode::default())),
        Arc::new(Mutex::new(clear::ClearArrayNode::default())),
        Arc::new(Mutex::new(find_item::FindItemInArrayNode::default())),
    ]
}
