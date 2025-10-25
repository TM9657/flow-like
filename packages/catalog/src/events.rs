pub mod api_event;
pub mod chat_event;
pub mod generic_event;
pub mod mail_event;
pub mod simple_event;

use flow_like::flow::node::NodeLogic;
use std::sync::Arc;

pub async fn register_functions() -> Vec<Arc<dyn NodeLogic>> {
    let mut output = vec![Arc::new(simple_event::SimpleEventNode::default()) as Arc<dyn NodeLogic>];
    output.append(&mut chat_event::register_functions().await);
    output.append(&mut generic_event::register_functions().await);
    output
}
