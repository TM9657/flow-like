pub mod chunk_text;

use flow_like::flow::node::NodeLogic;
use std::sync::Arc;

pub async fn register_functions() -> Vec<Arc<dyn NodeLogic>> {
    // vec![Arc::new(Mutex::new(chunk_text::ChunkTextNode::default())) as Arc<dyn NodeLogic>]
    vec![]
}
