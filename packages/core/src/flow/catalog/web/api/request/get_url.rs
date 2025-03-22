use crate::{
    flow::{
        catalog::web::api::HttpRequest,
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
pub struct GetUrlNode {}

impl GetUrlNode {
    pub fn new() -> Self {
        GetUrlNode {}
    }
}

#[async_trait]
impl NodeLogic for GetUrlNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "http_get_url",
            "Get Url",
            "Gets the url from a http request",
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

        node.add_output_pin("url", "Url", "The url of the request", VariableType::String);

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        let request: HttpRequest = context.evaluate_pin("request").await?;

        let url = request.url;

        context.set_pin_value("url", json!(url)).await?;

        Ok(())
    }
}
