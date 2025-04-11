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
pub struct GatherExecutionNode {}

impl GatherExecutionNode {
    pub fn new() -> Self {
        GatherExecutionNode {}
    }
}

#[async_trait]
impl NodeLogic for GatherExecutionNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "control_gather",
            "Gather",
            "Gather all execution states",
            "Control/Parallel",
        );
        node.add_icon("/flow/icons/par_execution.svg");

        node.add_input_pin("exec_in", "Input", "Input Pin", VariableType::Execution);
        node.add_input_pin("exec_in", "Input", "Input Pin", VariableType::Execution);

        node.add_output_pin(
            "exec_done",
            "In Sync",
            "Executes when all inputs are in Sync",
            VariableType::Execution,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_done").await?;
        let input_pins = context.get_pins_by_name("exec_in").await?;

        for pin in input_pins {
            let value: bool = match context.evaluate_pin_ref(pin).await {
                Ok(value) => value,
                Err(_) => {
                    // This means the pin is not set.
                    return Ok(());
                }
            };

            if !value {
                return Ok(());
            }
        }

        context.activate_exec_pin("exec_done").await?;

        return Ok(());
    }
}