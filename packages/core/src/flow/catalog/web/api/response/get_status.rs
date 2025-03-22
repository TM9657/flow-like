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
pub struct GetStatusNode {}

impl GetStatusNode {
    pub fn new() -> Self {
        GetStatusNode {}
    }
}

#[async_trait]
impl NodeLogic for GetStatusNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "http_response_get_status",
            "Get Status Code",
            "Gets the status code from a http response",
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
            "status_code",
            "Status Code",
            "The status code of the response",
            VariableType::Integer,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        let response: HttpResponse = context.evaluate_pin("response").await?;

        let status_code = response.get_status_code();

        context
            .set_pin_value("status_code", json!(status_code))
            .await?;

        Ok(())
    }
}
