use std::sync::Arc;

use flow_like::{
    flow::{
        execution::{LogLevel, context::ExecutionContext, internal_node::InternalNode},
        node::{Node, NodeLogic},
        pin::PinOptions,
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_types::{anyhow, async_trait, json::json, sync::Mutex, tokio};
use futures::future::join_all;
use rayon::prelude::*;

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
        node.add_input_pin("thread_model", "Threads", "Threads", VariableType::String)
            .set_default_value(Some(json!("tasks")))
            .set_options(
                PinOptions::new()
                    .set_valid_values(vec!["tasks".to_string(), "threads".to_string()])
                    .build(),
            );

        node.add_output_pin("exec_out", "Output", "Output Pin", VariableType::Execution);
        node.add_output_pin("exec_out", "Output", "Output Pin", VariableType::Execution);

        node.add_output_pin("exec_done", "Done", "Done Pin", VariableType::Execution);

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let exec_out_pins = context.get_pins_by_name("exec_out").await?;
        let use_threads: String = context.evaluate_pin("thread_model").await?;
        let use_threads = match use_threads.as_str() {
            "tasks" => false,
            "threads" => true,
            _ => false,
        };

        let mut parallel_items = vec![];

        for pin in exec_out_pins {
            let nodes = pin.lock().await.get_connected_nodes().await;
            for node in nodes {
                let context = context.create_sub_context(&node).await;
                parallel_items.push(context);
            }
        }

        if !use_threads {
            let results = join_all(
                parallel_items
                    .into_iter()
                    .map(|mut par_context| async move {
                        let run = InternalNode::trigger(&mut par_context, &mut None, true).await;
                        if let Err(err) = run {
                            par_context.log_message(
                                &format!("Error running node: {:?}", err),
                                LogLevel::Error,
                            );
                        }
                        par_context.end_trace();
                        par_context
                    }),
            )
            .await;

            for completed_context in results {
                context.push_sub_context(completed_context);
            }
        } else {
            let results = Arc::new(Mutex::new(Vec::new()));

            parallel_items.into_par_iter().for_each(|mut par_context| {
                let results_clone = Arc::clone(&results);

                let runtime = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(runtime) => runtime,
                    Err(err) => {
                        par_context.log_message(
                            &format!("Error creating runtime: {:?}", err),
                            LogLevel::Error,
                        );
                        par_context.end_trace();
                        return;
                    }
                };

                runtime.block_on(async {
                    let run = InternalNode::trigger(&mut par_context, &mut None, true).await;
                    if let Err(err) = run {
                        par_context.log_message(
                            &format!("Error running node: {:?}", err),
                            LogLevel::Error,
                        );
                    }
                    par_context.end_trace();

                    // Store the completed context
                    results_clone.lock().await.push(par_context);
                });
            });

            for completed_context in Arc::try_unwrap(results)
                .map_err(|_| anyhow!("Failed to unwrap Arc"))?
                .into_inner()
            {
                context.push_sub_context(completed_context);
            }
        }

        context.activate_exec_pin("exec_done").await?;

        return Ok(());
    }
}
