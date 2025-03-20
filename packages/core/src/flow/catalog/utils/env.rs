use crate::flow::node::NodeLogic;
use std::sync::Arc;
use tokio::sync::Mutex;

pub mod get_env;

pub async fn register_functions() -> Vec<Arc<dyn NodeLogic>> {
    vec![Arc::new(get_env::GetEnvVariableNode::default())]
}
