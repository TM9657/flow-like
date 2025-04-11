pub mod simple_event;
pub mod mail_event;
pub mod api_event;
pub mod chat_event;

use flow_like::flow::node::NodeLogic;
use std::sync::Arc;

pub async fn register_functions() -> Vec<Arc<dyn NodeLogic>> {
    vec![Arc::new(simple_event::SimpleEventNode::default()) as Arc<dyn NodeLogic>]
}
