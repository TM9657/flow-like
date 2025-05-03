use flow_like::{
    flow::{
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_types::json::json;
use flow_like_types::{Value, async_trait};

#[derive(Default)]
pub struct PushLocalSessionNode {}

impl PushLocalSessionNode {
    pub fn new() -> Self {
        PushLocalSessionNode {}
    }
}

#[async_trait]
impl NodeLogic for PushLocalSessionNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "events_chat_push_local_session",
            "Push Local Session",
            "Pushes a new local session to the chat. The session persists for one chat session.",
            "Events/Chat",
        );
        node.add_icon("/flow/icons/paperclip.svg");
        node.set_event_callback(true);

        node.add_input_pin(
            "exec_in",
            "Input",
            "Initiate Execution",
            VariableType::Execution,
        );

        node.add_input_pin(
            "local_session",
            "Local Session",
            "Generic Struct Type",
            VariableType::Struct,
        )
        .set_default_value(Some(json!({})));

        node.add_output_pin(
            "exec_out",
            "Output",
            "Done with the Execution",
            VariableType::Execution,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let session: Value = context
            .evaluate_pin("local_session")
            .await
            .unwrap_or(json!({}));

        context
            .stream_response("chat_local_session", session)
            .await?;
        context.activate_exec_pin("exec_out").await?;

        return Ok(());
    }
}
