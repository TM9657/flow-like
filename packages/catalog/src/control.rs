pub mod branch_node;
pub mod delay;
pub mod for_each;
pub mod par_execution;
pub mod reroute;
pub mod sequence;

use flow_like::flow::node::NodeLogic;
use std::sync::Arc;

pub async fn register_functions() -> Vec<Arc<dyn NodeLogic>> {
    vec![
        Arc::new(branch_node::BranchNode::default()),
        Arc::new(for_each::LoopNode::default()),
        Arc::new(sequence::SequenceNode::default()),
        Arc::new(par_execution::ParallelExecutionNode::default()),
        Arc::new(delay::DelayNode::default()),
        Arc::new(reroute::RerouteNode::default()),
    ]
}
