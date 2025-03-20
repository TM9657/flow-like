use crate::{
    flow::{
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        pin::PinOptions,
        variable::VariableType,
    },
    models::history::History,
    state::FlowLikeState,
};
use async_trait::async_trait;
use serde_json::json;

#[derive(Default)]
pub struct SetHistoryUserNode {}

impl SetHistoryUserNode {
    pub fn new() -> Self {
        SetHistoryUserNode {}
    }
}

#[async_trait]
impl NodeLogic for SetHistoryUserNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "ai_generative_set_history_user",
            "Set History User",
            "Sets the user attribute in a ChatHistory",
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

        node.add_input_pin("user", "User", "User Value", VariableType::String);

        node.add_output_pin(
            "exec_out",
            "Output",
            "Done with the Execution",
            VariableType::Execution,
        );

        node.add_output_pin(
            "history_out",
            "History",
            "Updated ChatHistory",
            VariableType::Struct,
        )
        .set_schema::<History>();

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        let mut history: History = context.evaluate_pin("history").await?;
        let user: String = context.evaluate_pin("user").await?;

        history.user = Some(user);

        context.set_pin_value("history_out", json!(history)).await?;
        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }
}
