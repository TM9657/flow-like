pub mod child;
pub mod extension;
pub mod filename;
pub mod parent;
pub mod raw;
pub mod set_extension;

use crate::flow::node::NodeLogic;
use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn register_functions() -> Vec<Arc<Mutex<dyn NodeLogic>>> {
    vec![
        Arc::new(Mutex::new(child::ChildNode::default())),
        Arc::new(Mutex::new(extension::ExtensionNode::default())),
        Arc::new(Mutex::new(filename::FilenameNode::default())),
        Arc::new(Mutex::new(parent::ParentNode::default())),
        Arc::new(Mutex::new(raw::RawPathNode::default())),
        Arc::new(Mutex::new(set_extension::SetExtensionNode::default())),
    ]
}
