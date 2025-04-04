use crate::{
    flow::{
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        pin::PinOptions,
        variable::VariableType,
    },
    state::FlowLikeState,
};
use async_trait::async_trait;
use flow_like_model_provider::history::History;
use serde_json::json;
#[derive(Default)]
pub struct SetHistoryStopWordsNode {}

impl SetHistoryStopWordsNode {
    pub fn new() -> Self {
        SetHistoryStopWordsNode {}
    }
}

#[async_trait]
impl NodeLogic for SetHistoryStopWordsNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "ai_generative_set_history_stop_words",
            "Set Stop Words",
            "Sets the stop_words attribute in a ChatHistory",
            "AI/Generative/History",
        );
        node.add_icon("/flow/icons/history-stop-words.svg");

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
            "stop_words",
            "Stop Words",
            "Stop Words Value",
            VariableType::String,
        )
        .set_value_type(crate::flow::pin::ValueType::Array);

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
        let stop_words: Vec<String> = context.evaluate_pin("stop_words").await?;

        history.stop = Some(stop_words);

        context.set_pin_value("history_out", json!(history)).await?;
        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }
}
