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

use crate::web::api::HttpResponse;

#[derive(Default)]
pub struct ToTextNode {}

impl ToTextNode {
    pub fn new() -> Self {
        ToTextNode {}
    }
}

#[async_trait]
impl NodeLogic for ToTextNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "http_response_to_text",
            "To Text",
            "Gets the body of a http response as text",
            "Web/API/Response",
        );
        node.add_icon("/flow/icons/web.svg");

        node.add_input_pin("exec_in", "", "", VariableType::Execution);

        node.add_input_pin(
            "response",
            "Response",
            "The http response",
            VariableType::Struct,
        )
        .set_schema::<HttpResponse>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_output_pin(
            "exec_out",
            "Exec Out",
            "Called when the node is finished",
            VariableType::Execution,
        );

        node.add_output_pin(
            "text",
            "Text",
            "The body of the response as text",
            VariableType::String,
        );

        node.add_output_pin(
            "failed",
            "Failed",
            "Called when the node fails",
            VariableType::Execution,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.activate_exec_pin("failed").await?;
        context.deactivate_exec_pin("exec_out").await?;
        let response: HttpResponse = context.evaluate_pin("response").await?;

        let text = response.to_text()?;

        context.set_pin_value("text", json!(text)).await?;

        context.deactivate_exec_pin("failed").await?;
        context.activate_exec_pin("exec_out").await?;
        Ok(())
    }
}
