pub mod chunk;
pub mod last_content;
pub mod last_message;
pub mod make;
pub mod message;
pub mod push_chunk;
pub mod usage;

use crate::flow::node::NodeLogic;
use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn register_functions() -> Vec<Arc<dyn NodeLogic>> {
    vec![
        Arc::new(push_chunk::PushChunkNode::default()),
        Arc::new(make::MakeResponseNode::default()),
        Arc::new(last_message::LastMessageNode::default()),
        Arc::new(last_content::LastContentNode::default()),
        Arc::new(message::get_content::GetContentNode::default()),
        Arc::new(message::get_role::GetRoleNode::default()),
        Arc::new(chunk::get_token::GetTokenNode::default()),
    ]
}
