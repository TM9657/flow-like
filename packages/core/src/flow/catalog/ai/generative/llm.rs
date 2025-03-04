pub mod find_llm;
pub mod invoke;
pub mod preferences;

use crate::flow::node::NodeLogic;
use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn register_functions() -> Vec<Arc<Mutex<dyn NodeLogic>>> {
    vec![
        Arc::new(Mutex::new(find_llm::FindLLMNode::default())),
        Arc::new(Mutex::new(invoke::InvokeLLM::default())),
        Arc::new(Mutex::new(preferences::make::MakePreferencesNode::default())),
        Arc::new(Mutex::new(preferences::hint::SetModelHintNode::default())),
        Arc::new(Mutex::new(preferences::weight::SetWeightNode::default())),
    ]
}
