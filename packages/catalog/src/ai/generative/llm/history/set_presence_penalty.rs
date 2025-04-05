use flow_like::{
    flow::{
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        pin::PinOptions,
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_model_provider::history::History;
use flow_like_types::{async_trait, json::json};

#[derive(Default)]
pub struct SetHistoryPresencePenaltyNode {}

impl SetHistoryPresencePenaltyNode {
    pub fn new() -> Self {
        SetHistoryPresencePenaltyNode {}
    }
}

#[async_trait]
impl NodeLogic for SetHistoryPresencePenaltyNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "ai_generative_set_history_presence_penalty",
            "Set History Presence Penalty",
            "Sets the presence_penalty attribute in a ChatHistory",
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

        node.add_input_pin(
            "presence_penalty",
            "Presence Penalty",
            "Presence Penalty Value",
            VariableType::Float,
        )
        .set_options(PinOptions::new().set_range((0.0, 1.0)).build());

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

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let mut history: History = context.evaluate_pin("history").await?;
        let presence_penalty: f64 = context.evaluate_pin("presence_penalty").await?;

        history.presence_penalty = Some(presence_penalty as f32);

        context.set_pin_value("history_out", json!(history)).await?;
        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }
}
