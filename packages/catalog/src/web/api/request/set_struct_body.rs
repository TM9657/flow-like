use std::{collections::HashSet, sync::Arc};
use flow_like::{flow::{execution::{context::ExecutionContext, internal_node::InternalNode, log::LogMessage, LogLevel}, node::{Node, NodeLogic}, pin::{PinOptions, ValueType}, variable::VariableType}, state::FlowLikeState};
use flow_like_types::{async_trait, json::json, reqwest, sync::{DashMap, Mutex}, Value};

use crate::{storage::path::FlowPath, web::api::{HttpBody, HttpRequest, HttpResponse, Method}};

#[derive(Default)]
pub struct SetStructBodyNode {}

impl SetStructBodyNode {
    pub fn new() -> Self {
        SetStructBodyNode {}
    }
}

#[async_trait]
impl NodeLogic for SetStructBodyNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "http_set_struct_body",
            "Set Struct Body",
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
            VariableType::Struct,
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

    async fn run(&self, context: &mut ExecutionContext) ->flow_like_types::Result<()> {
        let mut request: HttpRequest = context.evaluate_pin("request").await?;
        let body: Value = context.evaluate_pin("body").await?;

        request.body = Some(HttpBody::Json(body));

        context.set_pin_value("request", json!(request)).await?;

        Ok(())
    }
}
