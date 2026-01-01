use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic, NodeScores},
    pin::PinOptions,
    variable::VariableType,
};
use flow_like_model_provider::response::ResponseMessage;
use flow_like_types::{async_trait, json::json};

#[crate::register_node]
#[derive(Default)]
pub struct GetContentNode {}

impl GetContentNode {
    pub fn new() -> Self {
        GetContentNode {}
    }
}

#[async_trait]
impl NodeLogic for GetContentNode {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "ai_generative_llm_response_message_get_content",
            "Get Content",
            "Extracts the text content field from a response message",
            "AI/Generative/Response/Message",
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
            "message",
            "Message",
            "Message to extract content from",
            VariableType::Struct,
        )
        .set_schema::<ResponseMessage>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_output_pin(
            "content",
            "Content",
            "Content string from the message",
            VariableType::String,
        );

        node.add_output_pin(
            "success",
            "Success",
            "Whether content was successfully extracted",
            VariableType::Boolean,
        );

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let message: ResponseMessage = context.evaluate_pin("message").await?;
        let content = message.content.unwrap_or_default();
        let success = !content.is_empty();
        context.set_pin_value("content", json!(content)).await?;
        context.set_pin_value("success", json!(success)).await?;

        Ok(())
    }
}
