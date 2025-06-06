use flow_like::{
    flow::{
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        pin::PinOptions,
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_model_provider::response::Response;
use flow_like_types::{async_trait, json::json};

#[derive(Default)]
pub struct MakeResponseNode {}

impl MakeResponseNode {
    pub fn new() -> Self {
        MakeResponseNode {}
    }
}

#[async_trait]
impl NodeLogic for MakeResponseNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "ai_generative_llm_response_make",
            "Make Response",
            "",
            "AI/Generative/Response",
        );
        node.add_icon("/flow/icons/history.svg");

        node.add_output_pin("response", "Response", "", VariableType::Struct)
            .set_schema::<Response>()
            .set_options(PinOptions::new().set_enforce_schema(true).build());

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let response = Response::new();

        context.set_pin_value("response", json!(response)).await?;

        Ok(())
    }
}
