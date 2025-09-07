pub mod cache_dir;
pub mod storage_dir;
pub mod upload_dir;
pub mod user_dir;
pub mod virtual_dir;

use flow_like::flow::node::NodeLogic;
use std::sync::Arc;

pub async fn register_functions() -> Vec<Arc<dyn NodeLogic>> {
    vec![
        Arc::new(cache_dir::PathFromCacheDirNode::default()),
        Arc::new(storage_dir::PathFromStorageDirNode::default()),
        Arc::new(upload_dir::PathFromUploadDirNode::default()),
        Arc::new(user_dir::PathFromUserDirNode::default()),
        Arc::new(virtual_dir::VirtualDirNode::default()),
    ]
}
