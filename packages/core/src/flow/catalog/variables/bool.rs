use crate::flow::node::NodeLogic;
use std::sync::Arc;
use tokio::sync::Mutex;

pub mod logic;

pub async fn register_functions() -> Vec<Arc<Mutex<dyn NodeLogic>>> {
    vec![
        Arc::new(Mutex::new(logic::BoolAnd::default())),
        Arc::new(Mutex::new(logic::BoolOr::default())),
    ]
}
