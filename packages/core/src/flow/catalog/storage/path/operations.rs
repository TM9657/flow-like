pub mod changed;
pub mod copy;
pub mod delete;
pub mod exists;
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

use crate::flow::node::NodeLogic;
use std::sync::Arc;

pub async fn register_functions() -> Vec<Arc<dyn NodeLogic>> {
    vec![
        Arc::new(copy::CopyNode::default()),
        Arc::new(delete::DeleteNode::default()),
        Arc::new(get::GetNode::default()),
        Arc::new(get_range::GetRangeNode::default()),
        Arc::new(hash::HashFileNode::default()),
        Arc::new(head::HeadNode::default()),
        Arc::new(list_paths::ListPathsNode::default()),
        Arc::new(list_with_offset::ListWithOffsetNode::default()),
        Arc::new(put::PutNode::default()),
        Arc::new(rename::RenameNode::default()),
        Arc::new(sign_url::SignUrlNode::default()),
        Arc::new(sign_urls::SignUrlsNode::default()),
        Arc::new(exists::PathExistsNode::default()),
    ]
}
