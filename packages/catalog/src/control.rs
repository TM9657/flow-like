pub mod branch_node;
pub mod for_each;
pub mod par_execution;
pub mod sequence;
pub mod delay;

use flow_like::flow::node::NodeLogic;
use std::sync::Arc;

pub async fn register_functions() -> Vec<Arc<dyn NodeLogic>> {
    vec![
        Arc::new(branch_node::BranchNode::default()),
        Arc::new(for_each::LoopNode::default()),
        Arc::new(sequence::SequenceNode::default()),
        Arc::new(par_execution::ParallelExecutionNode::default()),
        Arc::new(delay::DelayNode::default()),
    ]
}
