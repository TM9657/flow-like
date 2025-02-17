use std::sync::Arc;

use crate::{
    flow::{
        board::Board,
        execution::{
            context::ExecutionContext, internal_node::InternalNode, log::LogMessage, LogLevel,
        },
        node::{Node, NodeLogic, NodeState},
        pin::{PinOptions, ValueType},
        utils::evaluate_pin_value,
        variable::VariableType,
    },
    state::FlowLikeState,
};
use async_trait::async_trait;

#[derive(Default)]
pub struct LoopNode {
    i: u64,
    length: u64,
}

impl LoopNode {
    pub fn new() -> Self {
        LoopNode { i: 0, length: 0 }
    }
}

#[async_trait]
impl NodeLogic for LoopNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "control_for_each",
            "For Each",
            "Loops over an Array",
            "Control",
        );
        node.add_icon("/flow/icons/for-each.svg");

        node.add_input_pin("exec_in", "Input", "Trigger Pin", VariableType::Execution);
        node.add_input_pin("array", "Array", "Array to Loop", VariableType::Generic)
            .set_value_type(crate::flow::pin::ValueType::Array)
            .set_options(
                PinOptions::new()
                    .set_enforce_generic_value_type(true)
                    .build(),
            );

        node.add_output_pin(
            "exec_out",
            "For Each Element",
            "Executes the current item",
            VariableType::Execution,
        );
        node.add_output_pin(
            "value",
            "Value",
            "The current item Value",
            VariableType::Generic,
        );
        node.add_output_pin(
            "index",
            "Index",
            "Current Array Index",
            VariableType::Integer,
        );

        node.add_output_pin(
            "done",
            "Done",
            "Executes once the array is dealt with.",
            VariableType::Execution,
        );

        return node;
    }

    async fn run(&mut self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        let array = context.get_pin_by_name("array").await?;
        let value = context.get_pin_by_name("value").await?;
        let exec_item = context.get_pin_by_name("exec_out").await?;
        let index = context.get_pin_by_name("index").await?;
        let done = context.get_pin_by_name("done").await?;

        let array_value = evaluate_pin_value(array).await?;
        let array_value = array_value
            .as_array()
            .ok_or(anyhow::anyhow!("Array value is not an array"))?;

        self.length = array_value.len() as u64;

        for (i, item) in array_value.iter().enumerate() {
            self.i = i as u64;
            let item = item.clone();
            let item = item.to_owned();
            value.lock().await.set_value(item).await;
            index
                .lock()
                .await
                .set_value(serde_json::json!(i as u64))
                .await;
            context.activate_exec_pin_ref(&exec_item).await?;
            let flow = exec_item.lock().await.get_connected_nodes().await;

            for node in flow {
                let mut sub_context = context.create_sub_context(&node).await;
                let mut log =
                    LogMessage::new(&format!("Triggered iteration: {}", i), LogLevel::Info, None);
                let run = InternalNode::trigger(&mut sub_context, &mut None, true).await;
                log.end();
                sub_context.log(log);
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

        context.activate_exec_pin_ref(&done).await?;

        return Ok(());
    }

    async fn get_progress(&self, context: &mut ExecutionContext) -> i32 {
        let state = context.get_state();

        match state {
            NodeState::Running => return ((self.i as f64 / self.length as f64) * 100.0) as i32,
            NodeState::Success => return 100,
            NodeState::Error => return 0,
            _ => return 0,
        }
    }

    async fn on_update(&self, node: &mut Node, board: Arc<Board>) {
        let match_type = node.match_type("array", board.clone(), Some(ValueType::Array));

        if match_type.is_err() {
            eprintln!("Error: {:?}", match_type.err());
        }

        let match_type = node.match_type("value", board, Some(ValueType::Normal));

        if match_type.is_err() {
            eprintln!("Error: {:?}", match_type.err());
        }

        let array_pin = node.get_pin_by_name("array").unwrap();
        if array_pin.data_type != VariableType::Generic {
            node.get_pin_mut_by_name("value").unwrap().data_type = array_pin.data_type.clone();
        }
    }
}
