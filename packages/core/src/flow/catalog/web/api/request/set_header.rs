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
pub struct SetHeaderNode {}

impl SetHeaderNode {
    pub fn new() -> Self {
        SetHeaderNode {}
    }
}

#[async_trait]
impl NodeLogic for SetHeaderNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "http_set_header",
            "Set Header",
            "Sets a header of a http request",
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
            "name",
            "Name",
            "The name of the header",
            VariableType::String,
        );

        node.add_input_pin(
            "value",
            "Value",
            "The value of the header",
            VariableType::String,
        );

        node.add_output_pin(
            "request",
            "Request",
            "The http request",
            VariableType::Struct,
        )
        .set_schema::<HttpRequest>();

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        let mut request: HttpRequest = context.evaluate_pin("request").await?;
        let name: String = context.evaluate_pin("name").await?;
        let value: String = context.evaluate_pin("value").await?;

        request.set_header(name, value);

        context.set_pin_value("request", json!(request)).await?;

        Ok(())
    }
}
