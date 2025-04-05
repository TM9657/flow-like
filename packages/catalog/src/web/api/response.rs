pub mod get_header;
pub mod get_headers;
pub mod get_status;
pub mod is_success;
pub mod to_bytes;
pub mod to_json;
pub mod to_text;

use flow_like::flow::node::NodeLogic;
use std::sync::Arc;

pub async fn register_functions() -> Vec<Arc<dyn NodeLogic>> {
    vec![
        Arc::new(get_status::GetStatusNode::default()),
        Arc::new(is_success::IsSuccessNode::default()),
        Arc::new(get_header::GetHeaderNode::default()),
        Arc::new(get_headers::GetHeadersNode::default()),
        Arc::new(to_json::ToJsonNode::default()),
        Arc::new(to_text::ToTextNode::default()),
        Arc::new(to_bytes::ToBytesNode::default()),
    ]
}
