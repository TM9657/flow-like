use flow_like::{
    flow::{
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        pin::PinOptions,
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_model_provider::response::{Response, ResponseMessage};
use flow_like_types::{async_trait, json::json};

#[crate::register_node]
#[derive(Default)]
pub struct LastMessageNode {}

impl LastMessageNode {
    pub fn new() -> Self {
        LastMessageNode {}
    }
}

#[async_trait]
impl NodeLogic for LastMessageNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "ai_generative_llm_response_last_message",
            "Last Message",
            "Extracts the last message from a Response",
            "AI/Generative/Response",
        );
        node.add_icon("/flow/icons/history.svg");

        node.add_input_pin(
            "response",
            "Response",
            "Response to extract from",
            VariableType::Struct,
        )
        .set_schema::<Response>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_output_pin(
            "message",
            "Message",
            "Last message from the response",
            VariableType::Struct,
        )
        .set_schema::<ResponseMessage>();

        node.add_output_pin(
            "success",
            "Success",
            "Whether a message was successfully extracted",
            VariableType::Boolean,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let response: Response = context.evaluate_pin("response").await?;

        if let Some(message) = response.last_message() {
            context.set_pin_value("message", json!(message)).await?;
            context.set_pin_value("success", json!(true)).await?;
        } else {
            context.set_pin_value("success", json!(false)).await?;
        }

        Ok(())
    }
}
