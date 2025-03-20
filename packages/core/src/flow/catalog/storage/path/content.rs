pub mod markdown_transform;
pub mod read_to_bytes;
pub mod read_to_string;
pub mod write_from_bytes;
pub mod write_from_string;

use crate::flow::node::NodeLogic;
use std::sync::Arc;

pub async fn register_functions() -> Vec<Arc<dyn NodeLogic>> {
    vec![
        Arc::new(read_to_bytes::ReadToBytesNode::default()),
        Arc::new(read_to_string::ReadToStringNode::default()),
        Arc::new(write_from_bytes::WriteBytesNode::default()),
        Arc::new(write_from_string::WriteStringNode::default()),
    ]
}
