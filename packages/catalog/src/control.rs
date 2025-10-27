pub mod branch_node;
pub mod call_ref;
pub mod delay;
pub mod do_n;
pub mod do_once;
pub mod flip_flop;
pub mod for_each;
pub mod for_each_with_break;
pub mod gate;
pub mod gather;
pub mod par_execution;
pub mod par_for_each;
pub mod reroute;
pub mod sequence;
pub mod while_loop;

use flow_like::flow::node::NodeLogic;
use std::sync::Arc;

pub async fn register_functions() -> Vec<Arc<dyn NodeLogic>> {
    vec![
        Arc::new(branch_node::BranchNode::default()),
        Arc::new(for_each::LoopNode::default()),
        Arc::new(par_for_each::ParLoopNode::default()),
        Arc::new(sequence::SequenceNode::default()),
        Arc::new(par_execution::ParallelExecutionNode),
        Arc::new(delay::DelayNode::default()),
        Arc::new(gather::GatherExecutionNode::default()),
        Arc::new(reroute::RerouteNode::default()),
        Arc::new(while_loop::WhileLoopNode::default()),
        Arc::new(call_ref::CallReferenceNode::default()),
        Arc::new(do_n::DoNNode::default()),
        Arc::new(do_once::DoOnceNode::default()),
        Arc::new(flip_flop::FlipFlopNode::default()),
        Arc::new(for_each_with_break::ForEachWithBreakNode::default()),
        Arc::new(gate::GateNode::default()),
    ]
}
