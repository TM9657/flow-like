use crate::{
    flow::{
        catalog::web::api::{HttpRequest, HttpResponse},
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        pin::PinOptions,
        variable::VariableType,
    },
    state::FlowLikeState,
};
use async_trait::async_trait;
use serde_json::json;

#[derive(Default)]
pub struct IsSuccessNode {}

impl IsSuccessNode {
    pub fn new() -> Self {
        IsSuccessNode {}
    }
}

#[async_trait]
impl NodeLogic for IsSuccessNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "http_response_is_success",
            "Is Success",
            "Checks if the status code of a http response is a success",
            "Web/API/Response",
        );
        node.add_icon("/flow/icons/web.svg");

        node.add_input_pin(
            "response",
            "Response",
            "The http response",
            VariableType::Struct,
        )
        .set_schema::<HttpResponse>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_output_pin(
            "is_success",
            "Is Success",
            "True if the status code is a success",
            VariableType::Boolean,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        let response: HttpResponse = context.evaluate_pin("response").await?;

        let is_success = response.is_success();

        context
            .set_pin_value("is_success", json!(is_success))
            .await?;

        Ok(())
    }
}
