use crate::{
    flow::{
        catalog::web::api::HttpResponse,
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        pin::PinOptions,
        variable::VariableType,
    },
    state::FlowLikeState,
};
use async_trait::async_trait;

#[derive(Default)]
pub struct ToJsonNode {}

impl ToJsonNode {
    pub fn new() -> Self {
        ToJsonNode {}
    }
}

#[async_trait]
impl NodeLogic for ToJsonNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "http_response_to_json",
            "To Struct",
            "Gets the body of a http response as json",
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
            "struct",
            "Struct",
            "The body of the response as json",
            VariableType::Struct,
        );

        node.add_output_pin(
            "failed",
            "Failed",
            "Called when the node fails",
            VariableType::Execution,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        context.activate_exec_pin("failed").await?;
        context.deactivate_exec_pin("exec_out").await?;
        let response: HttpResponse = context.evaluate_pin("response").await?;

        let json = response.to_json()?;

        context.set_pin_value("struct", json).await?;

        context.deactivate_exec_pin("failed").await?;
        context.activate_exec_pin("exec_out").await?;
        Ok(())
    }
}
