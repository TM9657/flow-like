pub mod child;
pub mod extension;
pub mod filename;
pub mod parent;
pub mod raw;
pub mod set_extension;

use crate::flow::node::NodeLogic;
use std::sync::Arc;

pub async fn register_functions() -> Vec<Arc<dyn NodeLogic>> {
    vec![
        Arc::new(child::ChildNode::default()),
        Arc::new(extension::ExtensionNode::default()),
        Arc::new(filename::FilenameNode::default()),
        Arc::new(parent::ParentNode::default()),
        Arc::new(raw::RawPathNode::default()),
        Arc::new(set_extension::SetExtensionNode::default()),
    ]
}
