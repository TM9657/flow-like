use crate::{
    flow::{
        execution::context::ExecutionContext, node::{Node, NodeLogic}, pin::PinOptions, variable::VariableType
    },
    models::history::History,
    state::FlowLikeState,
};
use async_trait::async_trait;
use serde_json::json;

#[derive(Default)]
pub struct SetHistoryNNode {}

impl SetHistoryNNode {
    pub fn new() -> Self {
        SetHistoryNNode {}
    }
}

#[async_trait]
impl NodeLogic for SetHistoryNNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "ai_generative_set_history_n",
            "Set History N",
            "Sets the n attribute in a ChatHistory",
            "AI/Generative/History",
        );
        node.add_icon("/flow/icons/history.svg");

        node.add_input_pin(
            "exec_in",
            "Input",
            "Initiate Execution",
            VariableType::Execution,
        );

        node.add_input_pin("history", "History", "ChatHistory", VariableType::Struct)
            .set_schema::<History>()
            .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_input_pin("n", "N", "N Value", VariableType::Integer);

        node.add_output_pin(
            "exec_out",
            "Output",
            "Done with the Execution",
            VariableType::Execution,
        );

        node.add_output_pin("history_out", "History", "Updated ChatHistory", VariableType::Struct)
            .set_schema::<History>();

        return node;
    }

    async fn run(&mut self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        let mut history: History = context.evaluate_pin("history").await?;
        let n: i64 = context.evaluate_pin("n").await?;

        history.n = Some(n as u32);

        context.set_pin_value("history_out", json!(history)).await?;
        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }
}