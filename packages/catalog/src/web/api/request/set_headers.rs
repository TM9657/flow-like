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

use crate::web::api::HttpRequest;

#[derive(Default)]
pub struct SetHeadersNode {}

impl SetHeadersNode {
    pub fn new() -> Self {
        SetHeadersNode {}
    }
}

#[async_trait]
impl NodeLogic for SetHeadersNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "http_set_headers",
            "Set Headers",
            "Sets the headers of a http request",
            "Web/API/Request",
        );
        node.add_icon("/flow/icons/web.svg");

        node.add_input_pin(
            "request",
            "Request",
            "The http request",
            VariableType::Struct,
        )
        .set_schema::<HttpRequest>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_input_pin(
            "headers",
            "Headers",
            "The headers of the request",
            VariableType::String,
        )
        .set_value_type(ValueType::HashMap);

        node.add_output_pin(
            "request",
            "Request",
            "The http request",
            VariableType::Struct,
        )
        .set_schema::<HttpRequest>();

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let mut request: HttpRequest = context.evaluate_pin("request").await?;
        let headers: std::collections::HashMap<String, String> =
            context.evaluate_pin("headers").await?;

        request.headers = Some(headers);

        context.set_pin_value("request", json!(request)).await?;

        Ok(())
    }
}
