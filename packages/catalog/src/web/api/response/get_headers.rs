use flow_like::{
    flow::{
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        pin::{PinOptions, ValueType},
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_types::{async_trait, json::json};

use crate::web::api::HttpResponse;
#[derive(Default)]
pub struct GetHeadersNode {}

impl GetHeadersNode {
    pub fn new() -> Self {
        GetHeadersNode {}
    }
}

#[async_trait]
impl NodeLogic for GetHeadersNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "http_response_get_headers",
            "Get Headers",
            "Gets all headers from a http request",
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
            "headers",
            "Headers",
            "The headers of the response",
            VariableType::String,
        )
        .set_value_type(ValueType::HashMap);

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let response: HttpResponse = context.evaluate_pin("response").await?;

        let headers = response.get_headers();

        context.set_pin_value("headers", json!(headers)).await?;

        Ok(())
    }
}
