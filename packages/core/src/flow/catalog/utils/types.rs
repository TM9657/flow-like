pub mod from_bytes;
pub mod from_string;
pub mod to_bytes;
pub mod to_string;
pub mod try_transform;
use crate::flow::node::NodeLogic;
use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn register_functions() -> Vec<Arc<Mutex<dyn NodeLogic>>> {
    vec![
        Arc::new(Mutex::new(try_transform::TryTransformNode::default())),
        Arc::new(Mutex::new(from_bytes::FromBytesNode::default())),
        Arc::new(Mutex::new(from_string::FromStringNode::default())),
        Arc::new(Mutex::new(to_bytes::ToBytesNode::default())),
        Arc::new(Mutex::new(to_string::ToStringNode::default())),
    ]
}
