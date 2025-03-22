use crate::{
    flow::{
        catalog::web::api::{HttpRequest, Method},
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
pub struct SetMethodNode {}

impl SetMethodNode {
    pub fn new() -> Self {
        SetMethodNode {}
    }
}

#[async_trait]
impl NodeLogic for SetMethodNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "http_set_method",
            "Set Method",
            "Sets the method of a http request",
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
            "method",
            "Method",
            "The method of the request",
            VariableType::String,
        )
        .set_options(
            PinOptions::new()
                .set_valid_values(vec![
                    "GET".to_string(),
                    "POST".to_string(),
                    "PUT".to_string(),
                    "DELETE".to_string(),
                    "PATCH".to_string(),
                ])
                .build(),
        )
        .set_default_value(Some(json!("GET")));

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
        let method: String = context.evaluate_pin("method").await?;

        request.method = match method.as_str() {
            "GET" => Method::GET,
            "POST" => Method::POST,
            "PUT" => Method::PUT,
            "DELETE" => Method::DELETE,
            "PATCH" => Method::PATCH,
            _ => return Err(anyhow::anyhow!("Invalid method")),
        };

        context.set_pin_value("request", json!(request)).await?;

        Ok(())
    }
}
