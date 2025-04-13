use flow_like::{
    flow::{
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        pin::PinOptions,
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_model_provider::history::{History, HistoryMessage};
use flow_like_types::{async_trait, json::json};

#[derive(Default)]
pub struct FromMessagesNode {}

impl FromMessagesNode {
    pub fn new() -> Self {
        FromMessagesNode {}
    }
}

#[async_trait]
impl NodeLogic for FromMessagesNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "ai_generative_from_messages",
            "From Messages",
            "Creates a Chat History from Messages",
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

        node.add_input_pin(
            "messages",
            "Messages",
            "Chat Messages",
            VariableType::Struct,
        )
        .set_schema::<HistoryMessage>()
        .set_value_type(flow_like::flow::pin::ValueType::Array)
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_output_pin("history", "History", "ChatHistory", VariableType::Struct)
            .set_schema::<History>()
            .set_options(PinOptions::new().set_enforce_schema(true).build());

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let messages: Vec<HistoryMessage> = context.evaluate_pin("messages").await?;
        let model_name: String = context.evaluate_pin("model_name").await?;
        let history = History::new(model_name, messages);

        context.set_pin_value("history", json!(history)).await?;

        Ok(())
    }
}
