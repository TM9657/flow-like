use flow_like::{
    flow::{
        execution::{LogLevel, context::ExecutionContext},
        node::{Node, NodeLogic},
        variable::VariableType,
    },
    state::{FlowLikeState, ToastLevel},
};
use flow_like_types::async_trait;

#[derive(Default)]
pub struct ErrorNode {}

impl ErrorNode {
    pub fn new() -> Self {
        ErrorNode {}
    }
}

#[async_trait]
impl NodeLogic for ErrorNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "log_error",
            "Log Error",
            "Logs / Prints an Error",
            "Logging",
        );
        node.add_icon("/flow/icons/log-error.svg");

        node.add_input_pin("exec_in", "Input", "Trigger Pin", VariableType::Execution);

        node.add_input_pin(
            "message",
            "Message",
            "Print Error Message",
            VariableType::String,
        )
        .set_default_value(Some(flow_like_types::json::json!("")));

        node.add_input_pin(
            "toast",
            "On Screen?",
            "Should the user see a toast popping up?",
            VariableType::Boolean,
        )
        .set_default_value(Some(flow_like_types::json::json!(false)));

        node.add_output_pin(
            "exec_out",
            "Output",
            "The flow to follow if the condition is true",
            VariableType::Execution,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let should_toast = context.evaluate_pin::<bool>("toast").await?;
        let message = context.evaluate_pin::<String>("message").await?;

        if should_toast {
            context.toast_message(&message, ToastLevel::Error).await?;
        }

        let output = context.get_pin_by_name("exec_out").await?;

        context.log_message(&message, LogLevel::Error);
        context.activate_exec_pin_ref(&output).await?;

        return Ok(());
    }
}
