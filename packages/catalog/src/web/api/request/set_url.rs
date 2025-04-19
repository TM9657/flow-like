use flow_like::{
    flow::{
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        pin::PinOptions,
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_types::{async_trait, json::json};

use crate::web::api::HttpRequest;

#[derive(Default)]
pub struct SetUrlNode {}

impl SetUrlNode {
    pub fn new() -> Self {
        SetUrlNode {}
    }
}

#[async_trait]
impl NodeLogic for SetUrlNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "http_set_url",
            "Set Url",
            "Sets the url of a http request",
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

        node.add_input_pin("url", "Url", "The url of the request", VariableType::String);

        node.add_output_pin(
            "request_out",
            "Request",
            "The http request",
            VariableType::Struct,
        )
        .set_schema::<HttpRequest>();

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let mut request: HttpRequest = context.evaluate_pin("request").await?;
        let url: String = context.evaluate_pin("url").await?;

        request.url = url;

        context.set_pin_value("request_out", json!(request)).await?;

        Ok(())
    }
}
