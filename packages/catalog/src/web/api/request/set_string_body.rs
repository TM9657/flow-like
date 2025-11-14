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

use crate::web::api::{HttpBody, HttpRequest};

#[crate::register_node]
#[derive(Default)]
pub struct SetStringBodyNode {}

impl SetStringBodyNode {
    pub fn new() -> Self {
        SetStringBodyNode {}
    }
}

#[async_trait]
impl NodeLogic for SetStringBodyNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "http_set_string_body",
            "Set String Body",
            "Sets the body of a http request",
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
            "body",
            "Body",
            "The body of the request",
            VariableType::String,
        );

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
        let body: String = context.evaluate_pin("body").await?;

        request.body = Some(HttpBody::String(body));

        context.set_pin_value("request_out", json!(request)).await?;

        Ok(())
    }
}
