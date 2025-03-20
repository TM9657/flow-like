pub mod from_bytes;
pub mod from_string;
pub mod to_bytes;
pub mod to_string;
pub mod try_transform;
use crate::flow::node::NodeLogic;
use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn register_functions() -> Vec<Arc<dyn NodeLogic>> {
    vec![
        Arc::new(try_transform::TryTransformNode::default()),
        Arc::new(from_bytes::FromBytesNode::default()),
        Arc::new(from_string::FromStringNode::default()),
        Arc::new(to_bytes::ToBytesNode::default()),
        Arc::new(to_string::ToStringNode::default()),
    ]
}
