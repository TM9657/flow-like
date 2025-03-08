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
pub struct SetHistoryTopPNode {}

impl SetHistoryTopPNode {
    pub fn new() -> Self {
        SetHistoryTopPNode {}
    }
}

#[async_trait]
impl NodeLogic for SetHistoryTopPNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "ai_generative_set_history_top_p",
            "Set History Top P",
            "Sets the top_p attribute in a ChatHistory",
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

        node.add_input_pin("top_p", "Top P", "Top P Value", VariableType::Float);

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
        let top_p: f64 = context.evaluate_pin("top_p").await?;

        history.top_p = Some(top_p as f32);

        context.set_pin_value("history_out", json!(history)).await?;
        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }
}