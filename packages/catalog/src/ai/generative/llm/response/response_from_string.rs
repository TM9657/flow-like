use flow_like::{
    flow::{
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        pin::PinOptions,
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_model_provider::response::{Choice, Response, ResponseMessage};
use flow_like_types::{async_trait, json::json};

#[crate::register_node]
#[derive(Default)]
pub struct ResponseFromStringNode {}

impl ResponseFromStringNode {
    pub fn new() -> Self {
        ResponseFromStringNode {}
    }
}

#[async_trait]
impl NodeLogic for ResponseFromStringNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "ai_generative_llm_response_from_string",
            "Response From String",
            "",
            "AI/Generative/Response",
        );
        node.add_icon("/flow/icons/history.svg");

        node.add_input_pin("content", "Content", "Content", VariableType::String)
            .set_default_value(Some(json!("")));

        node.add_output_pin("response", "Response", "", VariableType::Struct)
            .set_schema::<Response>()
            .set_options(PinOptions::new().set_enforce_schema(true).build());

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let mut response = Response::new();
        let message: String = context.evaluate_pin("content").await?;

        response.choices.push(Choice {
            finish_reason: String::from("artificial"),
            index: 0,
            logprobs: None,
            message: ResponseMessage {
                role: "assistant".to_string(),
                content: Some(message),
                ..Default::default()
            },
        });

        context.set_pin_value("response", json!(response)).await?;

        Ok(())
    }
}
