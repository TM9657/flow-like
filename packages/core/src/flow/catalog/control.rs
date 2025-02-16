pub mod branch_node;
pub mod for_each;
pub mod par_execution;
pub mod sequence;

use crate::flow::node::NodeLogic;
use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn register_functions() -> Vec<Arc<Mutex<dyn NodeLogic>>> {
    vec![
        Arc::new(Mutex::new(branch_node::BranchNode::default())),
        Arc::new(Mutex::new(for_each::LoopNode::default())),
        Arc::new(Mutex::new(sequence::SequenceNode::default())),
        Arc::new(Mutex::new(par_execution::ParallelExecutionNode::default())),
    ]
}
