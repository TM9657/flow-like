pub mod get_header;
pub mod get_headers;
pub mod get_method;
pub mod get_url;
pub mod make;
pub mod set_bytes_body;
pub mod set_header;
pub mod set_headers;
pub mod set_method;
pub mod set_string_body;
pub mod set_struct_body;
pub mod set_url;

use flow_like::flow::node::NodeLogic;
use std::sync::Arc;

pub async fn register_functions() -> Vec<Arc<dyn NodeLogic>> {
    vec![
        Arc::new(make::MakeRequestNode::default()),
        Arc::new(set_header::SetHeaderNode::default()),
        Arc::new(set_headers::SetHeadersNode::default()),
        Arc::new(get_header::GetHeaderNode::default()),
        Arc::new(get_headers::GetHeadersNode::default()),
        Arc::new(set_method::SetMethodNode::default()),
        Arc::new(set_url::SetUrlNode::default()),
        Arc::new(get_method::GetMethodNode::default()),
        Arc::new(get_url::GetUrlNode::default()),
        Arc::new(set_struct_body::SetStructBodyNode::default()),
        Arc::new(set_string_body::SetStringBodyNode::default()),
        Arc::new(set_bytes_body::SetBytesBodyNode::default()),
    ]
}
