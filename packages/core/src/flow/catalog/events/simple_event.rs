use crate::{
    flow::{
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        variable::VariableType,
    },
    state::FlowLikeState,
};
use async_trait::async_trait;

#[derive(Default)]
pub struct SimpleEventNode {}

impl SimpleEventNode {
    pub fn new() -> Self {
        SimpleEventNode {}
    }
}

#[async_trait]
impl NodeLogic for SimpleEventNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "events_simple",
            "Simple Event",
            "A simple event without input or output",
            "Events",
        );
        node.add_icon("/flow/icons/event.svg");
        node.set_start(true);

        node.add_output_pin(
            "exec_out",
            "Output",
            "Starting an event",
            VariableType::Execution,
        );
        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        let exec_out_pin = context.get_pin_by_name("exec_out").await?;

        context.activate_exec_pin_ref(&exec_out_pin).await?;

        return Ok(());
    }
}
