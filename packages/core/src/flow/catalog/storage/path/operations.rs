pub mod changed;
pub mod copy;
pub mod delete;
pub mod get;
pub mod get_range;
pub mod hash;
pub mod head;
pub mod list_paths;
pub mod list_with_offset;
pub mod put;
pub mod rename;
pub mod sign_url;
pub mod sign_urls;
pub mod exists;

use crate::flow::node::NodeLogic;
use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn register_functions() -> Vec<Arc<Mutex<dyn NodeLogic>>> {
    vec![
        Arc::new(Mutex::new(copy::CopyNode::default())),
        Arc::new(Mutex::new(delete::DeleteNode::default())),
        Arc::new(Mutex::new(get::GetNode::default())),
        Arc::new(Mutex::new(get_range::GetRangeNode::default())),
        Arc::new(Mutex::new(hash::HashFileNode::default())),
        Arc::new(Mutex::new(head::HeadNode::default())),
        Arc::new(Mutex::new(list_paths::ListPathsNode::default())),
        Arc::new(Mutex::new(list_with_offset::ListWithOffsetNode::default())),
        Arc::new(Mutex::new(put::PutNode::default())),
        Arc::new(Mutex::new(rename::RenameNode::default())),
        Arc::new(Mutex::new(sign_url::SignUrlNode::default())),
        Arc::new(Mutex::new(sign_urls::SignUrlsNode::default())),
        Arc::new(Mutex::new(exists::PathExistsNode::default())),
    ]
}
