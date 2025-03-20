pub mod error;
pub mod info;
pub mod trace;
pub mod warning;

use crate::flow::node::NodeLogic;
use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn register_functions() -> Vec<Arc<dyn NodeLogic>> {
    vec![
        Arc::new(error::ErrorNode::default()) as Arc<dyn NodeLogic>,
        Arc::new(info::InfoNode::default()) as Arc<dyn NodeLogic>,
        Arc::new(warning::WarningNode::default()) as Arc<dyn NodeLogic>,
    ]
}
