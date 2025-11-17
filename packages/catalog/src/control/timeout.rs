use flow_like::{
    flow::{
        execution::{LogLevel, context::ExecutionContext, internal_node::InternalNode},
        node::{Node, NodeLogic},
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_types::{async_trait, tokio::time};
use std::sync::Arc;

#[crate::register_node]
#[derive(Default)]
pub struct TimeoutNode {}

impl TimeoutNode {
    pub fn new() -> Self {
        TimeoutNode {}
    }

    async fn execute_nodes(sub_contexts: Vec<ExecutionContext>) -> Vec<ExecutionContext> {
        let mut completed = Vec::new();
        for mut sub in sub_contexts {
            if let Err(err) = InternalNode::trigger(&mut sub, &mut None, true).await {
                sub.log_message(
                    &format!("Error running node in timeout: {err:?}"),
                    LogLevel::Error,
                );
            }
            sub.end_trace();
            completed.push(sub);
        }
        completed
    }

    async fn create_interrupted_contexts(
        context: &mut ExecutionContext,
        nodes: &[Arc<InternalNode>],
    ) -> Vec<ExecutionContext> {
        let mut interrupted_contexts = Vec::new();
        for node in nodes {
            let mut sub = context.create_sub_context(node).await;
            sub.log_message("Execution was interrupted due to timeout", LogLevel::Warn);
            sub.end_trace();
            interrupted_contexts.push(sub);
        }
        interrupted_contexts
    }
}

#[async_trait]
impl NodeLogic for TimeoutNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "control_timeout",
            "Timeout",
            "Executes with a timeout, branching based on completion",
            "Control",
        );

        node.set_long_running(true);
        node.add_icon("/flow/icons/clock.svg");

        node.add_input_pin(
            "exec_in",
            "Execute",
            "Initiate Execution",
            VariableType::Execution,
        );
        node.add_input_pin(
            "timeout_ms",
            "Timeout (ms)",
            "Timeout duration in milliseconds",
            VariableType::Float,
        )
        .set_default_value(Some(flow_like_types::json::json!(5000.0)));

        node.add_output_pin(
            "exec_body",
            "Execute",
            "Execution path to run with timeout",
            VariableType::Execution,
        );

        node.add_output_pin(
            "exec_completed",
            "Completed",
            "Execution completed within timeout",
            VariableType::Execution,
        );

        node.add_output_pin(
            "exec_timed_out",
            "Timed Out",
            "Execution exceeded timeout duration",
            VariableType::Execution,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let timeout_duration: f64 = context.evaluate_pin("timeout_ms").await?;
        let timeout_duration = time::Duration::from_millis(timeout_duration as u64);

        let exec_body_pin = context.get_pin_by_name("exec_body").await?;
        let completed_pin = context.get_pin_by_name("exec_completed").await?;
        let timed_out_pin = context.get_pin_by_name("exec_timed_out").await?;

        context.deactivate_exec_pin_ref(&completed_pin).await?;
        context.deactivate_exec_pin_ref(&timed_out_pin).await?;
        context.activate_exec_pin_ref(&exec_body_pin).await?;

        let nodes = exec_body_pin.lock().await.get_connected_nodes().await;

        let (timed_out, sub_contexts) = if nodes.is_empty() {
            (false, Vec::new())
        } else {
            let mut sub_contexts = Vec::new();
            for node in &nodes {
                sub_contexts.push(context.create_sub_context(node).await);
            }

            let execution_task = Self::execute_nodes(sub_contexts);

            match time::timeout(timeout_duration, execution_task).await {
                Ok(completed) => (false, completed),
                Err(_) => {
                    let interrupted = Self::create_interrupted_contexts(context, &nodes).await;
                    (true, interrupted)
                }
            }
        };

        for mut sub in sub_contexts {
            context.push_sub_context(&mut sub);
        }

        context.deactivate_exec_pin_ref(&exec_body_pin).await?;

        if timed_out {
            context.log_message(
                &format!(
                    "Execution timed out after {} ms",
                    timeout_duration.as_millis()
                ),
                LogLevel::Warn,
            );
            context.activate_exec_pin_ref(&timed_out_pin).await?;
        } else {
            context.activate_exec_pin_ref(&completed_pin).await?;
        }

        Ok(())
    }
}
