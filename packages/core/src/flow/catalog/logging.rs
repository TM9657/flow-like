pub mod error;
pub mod info;
pub mod trace;
pub mod warning;

use crate::flow::node::NodeLogic;
use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn register_functions() -> Vec<Arc<Mutex<dyn NodeLogic>>> {
    vec![
        Arc::new(Mutex::new(error::ErrorNode::default())) as Arc<Mutex<dyn NodeLogic>>,
        Arc::new(Mutex::new(info::InfoNode::default())) as Arc<Mutex<dyn NodeLogic>>,
        Arc::new(Mutex::new(warning::WarningNode::default())) as Arc<Mutex<dyn NodeLogic>>,
    ]
}
