use flow_like::{
    flow::{
        execution::context::ExecutionContext,
        node::{Node, NodeLogic, NodeScores},
        pin::PinOptions,
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_model_provider::response::Response;
use flow_like_types::{async_trait, json::json};

#[crate::register_node]
#[derive(Default)]
pub struct LastContentNode {}

impl LastContentNode {
    pub fn new() -> Self {
        LastContentNode {}
    }
}

#[async_trait]
impl NodeLogic for LastContentNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "ai_generative_llm_response_last_content",
            "Last Content",
            "Extracts the content string from the last assistant message in a response",
            "AI/Generative/Response",
        );
        node.add_icon("/flow/icons/history.svg");
        node.set_scores(
            NodeScores::new()
                .set_privacy(10)
                .set_security(10)
                .set_performance(9)
                .set_reliability(10)
                .set_governance(9)
                .set_cost(10)
                .build(),
        );

        node.add_input_pin(
            "response",
            "Response",
            "LLM response to extract from",
            VariableType::Struct,
        )
        .set_schema::<Response>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_output_pin(
            "content",
            "Content",
            "Content string from the last message",
            VariableType::String,
        );

        node.add_output_pin(
            "success",
            "Success",
            "Whether content was successfully extracted",
            VariableType::Boolean,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let response: Response = context.evaluate_pin("response").await?;
        let content = response
            .last_message()
            .and_then(|message| message.content.clone())
            .unwrap_or_default();

        let success = !content.is_empty();
        context.set_pin_value("content", json!(content)).await?;
        context.set_pin_value("success", json!(success)).await?;

        Ok(())
    }
}
