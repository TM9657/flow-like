use flow_like::{
    flow::{
        execution::{context::ExecutionContext, internal_node::InternalNode, LogLevel},
        node::{Node, NodeLogic},
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_types::async_trait;
use futures::future::join_all;

#[derive(Default)]
pub struct ParallelExecutionNode {}

impl ParallelExecutionNode {
    pub fn new() -> Self {
        ParallelExecutionNode {}
    }
}

#[async_trait]
impl NodeLogic for ParallelExecutionNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "control_par_execution",
            "Parallel Execution",
            "Parallel Execution",
            "Control",
        );
        node.add_icon("/flow/icons/par_execution.svg");

        node.add_input_pin("exec_in", "Input", "Trigger Pin", VariableType::Execution);
        node.add_output_pin("exec_out", "Output", "Output Pin", VariableType::Execution);
        node.add_output_pin("exec_out", "Output", "Output Pin", VariableType::Execution);

        node.add_output_pin(
            "exec_done",
            "Done",
            "Done Pin",
            VariableType::Execution,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let exec_out_pins = context.get_pins_by_name("exec_out").await?;

        let mut parallel_items = vec![];

        for pin in exec_out_pins {
            let nodes = pin.lock().await.get_connected_nodes().await;
            for node in nodes {
                let context = context.create_sub_context(&node).await;
                parallel_items.push(context);
            }
        }

        let results = join_all(parallel_items.into_iter().map(|mut par_context| async move {
            let run = InternalNode::trigger(&mut par_context, &mut None, true).await;
            if let Err(err) = run {
                par_context.log_message(&format!(
                    "Error running node: {:?}",
                    err
                ), LogLevel::Error);
            }
            par_context.end_trace();
            par_context
        }))
        .await;

        for completed_context in results {
            context.push_sub_context(completed_context);
        }

        context.activate_exec_pin("exec_done").await?;

        return Ok(());
    }
}


// async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
//     let exec_out_pins = context.get_pins_by_name("exec_out").await?;
//     for pin in exec_out_pins {
//         let deactivate_pin = context.activate_exec_pin_ref(&pin).await;
//         if let Err(err) = deactivate_pin {
//             eprintln!("Error activating pin: {:?}", err);
//         }
//     }

//     return Ok(());
// }