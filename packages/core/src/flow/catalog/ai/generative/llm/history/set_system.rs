use crate::{
    flow::{
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        pin::PinOptions,
        variable::VariableType,
    },
    state::FlowLikeState,
};
use async_trait::async_trait;
use flow_like_model_provider::{
    history::{History, HistoryMessage, Role},
    llm::LLMCallback,
    response::{Response, ResponseMessage},
    response_chunk::ResponseChunk,
};
use serde_json::json;

#[derive(Default)]
pub struct SetSystemPromptMessageNode {}

impl SetSystemPromptMessageNode {
    pub fn new() -> Self {
        SetSystemPromptMessageNode {}
    }
}

#[async_trait]
impl NodeLogic for SetSystemPromptMessageNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "ai_generative_set_system_prompt_message",
            "Set System Message",
            "Sets the system prompt message in a ChatHistory",
            "AI/Generative/History",
        );
        node.add_icon("/flow/icons/history-set.svg");

        node.add_input_pin(
            "exec_in",
            "Input",
            "Initiate Execution",
            VariableType::Execution,
        );

        node.add_input_pin("history", "History", "ChatHistory", VariableType::Struct)
            .set_schema::<History>()
            .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_input_pin(
            "message",
            "Message",
            "System Prompt Message",
            VariableType::String,
        )
        .set_default_value(Some(json!("")));

        node.add_output_pin(
            "exec_out",
            "Output",
            "Done with the Execution",
            VariableType::Execution,
        );

        node.add_output_pin(
            "history_out",
            "History",
            "Updated ChatHistory",
            VariableType::Struct,
        )
        .set_schema::<History>();

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        let mut history: History = context.evaluate_pin("history").await?;
        let new_message: String = context.evaluate_pin("message").await?;

        let new_message = HistoryMessage::from_string(Role::System, &new_message);

        history
            .messages
            .retain(|message| message.role != Role::System);
        history.messages.insert(0, new_message);

        context.set_pin_value("history_out", json!(history)).await?;
        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }
}
