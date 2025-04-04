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
pub struct MakeHistoryNode {}

impl MakeHistoryNode {
    pub fn new() -> Self {
        MakeHistoryNode {}
    }
}

#[async_trait]
impl NodeLogic for MakeHistoryNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "ai_generative_make_history",
            "Make History",
            "Creates a ChatHistory struct",
            "AI/Generative/History",
        );
        node.add_icon("/flow/icons/history.svg");

        node.add_input_pin(
            "model_name",
            "Model Name",
            "Model Name",
            VariableType::String,
        )
        .set_default_value(Some(json!("")));

        node.add_output_pin("history", "History", "ChatHistory", VariableType::Struct)
            .set_schema::<History>()
            .set_options(PinOptions::new().set_enforce_schema(true).build());

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        let model_name: String = context.evaluate_pin("model_name").await?;
        let history = History::new(model_name, vec![]);

        context.set_pin_value("history", json!(history)).await?;

        Ok(())
    }
}
