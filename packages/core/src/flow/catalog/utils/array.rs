use crate::flow::node::NodeLogic;
use std::sync::Arc;
use tokio::sync::Mutex;

pub mod get;
pub mod includes;
pub mod make;
pub mod pop;
pub mod push;
pub mod set;

pub async fn register_functions() -> Vec<Arc<Mutex<dyn NodeLogic>>> {
    vec![
        Arc::new(Mutex::new(get::GetArrayElementNode::default())),
        Arc::new(Mutex::new(includes::ArrayIncludesNode::default())),
        Arc::new(Mutex::new(make::MakeArrayNode::default())),
        Arc::new(Mutex::new(pop::PopArrayNode::default())),
        Arc::new(Mutex::new(push::PushArrayNode::default())),
        Arc::new(Mutex::new(set::SetIndexArrayNode::default())),
    ]
}
