use flow_like::{
    flow::{
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_types::async_trait;

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

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let exec_out_pins = context.get_pins_by_name("exec_out").await?;
        for pin in exec_out_pins {
            let deactivate_pin = context.activate_exec_pin_ref(&pin).await;
            if let Err(err) = deactivate_pin {
                eprintln!("Error activating pin: {:?}", err);
            }
        }

        return Ok(());
    }
}
