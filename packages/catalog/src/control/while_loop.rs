
use flow_like::{
    flow::{
        execution::{LogLevel, context::ExecutionContext, internal_node::InternalNode},
        node::{Node, NodeLogic},
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_types::{async_trait, json::json};

#[derive(Default)]
pub struct WhileLoopNode {}

impl WhileLoopNode {
    pub fn new() -> Self {
        WhileLoopNode {}
    }
}

#[async_trait]
impl NodeLogic for WhileLoopNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "while_loop",
            "While Loop",
            "Loop downstream execution in while loop",
            "Control",
        );
        node.add_icon("/flow/icons/for-each.svg");

        // inputs
        node.add_input_pin("exec_in", "Input", "Trigger Pin", VariableType::Execution);
        
        node.add_input_pin(
            "condition",
            "Condition",
            "Loop while this is true",
            VariableType::Boolean,
        )
        .set_default_value(Some(flow_like_types::json::json!(false)));

        node.add_input_pin(
            "max_iter",
            "Max",
            "Maximum number of iterations",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(15)));

        // outputs
        node.add_output_pin(
            "exec_out",
            "Downstream Execution",
            "Downstream execution propagation",
            VariableType::Execution,
        );

        node.add_output_pin(
            "iter",
            "Iter",
            "Current iteration index",
            VariableType::Integer,
        );

        node.add_output_pin(
            "done",
            "Done",
            "Executes once loop terminates",
            VariableType::Execution,
        );

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {

        let exec_item = context.get_pin_by_name("exec_out").await?;
        let iter = context.get_pin_by_name("iter").await?;
        let done = context.get_pin_by_name("done").await?;
        let max_iter: u64 = context.evaluate_pin("max_iter").await?;

        context.activate_exec_pin_ref(&exec_item).await?;
        for i in 0..max_iter {
            let condition = context.evaluate_pin::<bool>("condition").await?;
            if !condition {
                break;
            }
            iter
                .lock()
                .await
                .set_value(flow_like_types::json::json!(i))
                .await;
            let flow = exec_item.lock().await.get_connected_nodes().await;
            for node in flow {
                let mut sub_context = context.create_sub_context(&node).await;
                let run = InternalNode::trigger(&mut sub_context, &mut None, true).await;
                sub_context.end_trace();
                context.push_sub_context(sub_context);

                if run.is_err() {
                    let error = run.err().unwrap();
                    context.log_message(
                        &format!("Error: {:?} in iteration {}", error, i),
                        LogLevel::Error,
                    );
                }
            }
        }

        context.deactivate_exec_pin_ref(&exec_item).await?;
        context.activate_exec_pin_ref(&done).await?;
        Ok(())
    }
}
