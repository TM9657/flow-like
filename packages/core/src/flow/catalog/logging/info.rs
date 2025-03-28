use crate::{
    flow::{
        execution::{context::ExecutionContext, LogLevel},
        node::{Node, NodeLogic},
        variable::VariableType,
    },
    state::FlowLikeState,
};
use async_trait::async_trait;

#[derive(Default)]
pub struct InfoNode {}

impl InfoNode {
    pub fn new() -> Self {
        InfoNode {}
    }
}

#[async_trait]
impl NodeLogic for InfoNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "log_info",
            "Print Info",
            "Print Debugging Information",
            "Logging",
        );
        node.add_icon("/flow/icons/log-info.svg");

        node.add_input_pin("exec_in", "Input", "Trigger Pin", VariableType::Execution);

        node.add_input_pin(
            "message",
            "Message",
            "The message to log",
            VariableType::String,
        )
        .set_default_value(Some(serde_json::json!("")));

        node.add_input_pin(
            "toast",
            "On Screen?",
            "Should the user see a toast popping up?",
            VariableType::Boolean,
        )
        .set_default_value(Some(serde_json::json!(false)));

        node.add_output_pin(
            "exec_out",
            "Output",
            "The flow to follow if the condition is true",
            VariableType::Execution,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        let should_toast = context.evaluate_pin::<bool>("toast").await?;
        let message = context.evaluate_pin::<String>("message").await?;

        if should_toast {
            context
                .toast_message(&message, crate::state::ToastLevel::Info)
                .await?;
        }

        let output = context.get_pin_by_name("exec_out").await?;

        context.log_message(&message, LogLevel::Info);
        context.activate_exec_pin_ref(&output).await?;

        return Ok(());
    }
}
