use std::{collections::HashSet, sync::Arc};
use flow_like::{flow::{execution::{context::ExecutionContext, internal_node::InternalNode, log::LogMessage, LogLevel}, node::{Node, NodeLogic}, pin::{PinOptions, ValueType}, variable::VariableType}, state::FlowLikeState};
use flow_like_types::{async_trait, reqwest, sync::{DashMap, Mutex}, json::json};

use crate::{storage::path::FlowPath, web::api::HttpResponse};

#[derive(Default)]
pub struct GetHeaderNode {}

impl GetHeaderNode {
    pub fn new() -> Self {
        GetHeaderNode {}
    }
}

#[async_trait]
impl NodeLogic for GetHeaderNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "http_response_get_header",
            "Get Header",
            "Gets a header from a http request",
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

        node.add_input_pin(
            "header",
            "Header",
            "The header to get",
            VariableType::String,
        );

        node.add_output_pin(
            "found",
            "Found",
            "True if the header was found",
            VariableType::Boolean,
        );

        node.add_output_pin(
            "value",
            "Value",
            "The value of the header",
            VariableType::String,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) ->flow_like_types::Result<()> {
        let response: HttpResponse = context.evaluate_pin("response").await?;
        let header: String = context.evaluate_pin("header").await?;

        let value = response.get_header(&header);

        context
            .set_pin_value("found", json!(value.is_some()))
            .await?;
        context
            .set_pin_value("value", json!(value.unwrap_or(&"".to_string())))
            .await?;

        Ok(())
    }
}
