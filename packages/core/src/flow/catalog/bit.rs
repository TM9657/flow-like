pub mod from_string;

use crate::flow::node::NodeLogic;
use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn register_functions() -> Vec<Arc<Mutex<dyn NodeLogic>>> {
    vec![
        Arc::new(Mutex::new(from_string::BitFromStringNode::default())) as Arc<Mutex<dyn NodeLogic>>,
    ]
}
