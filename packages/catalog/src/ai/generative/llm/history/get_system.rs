use flow_like::{
    flow::{
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        pin::PinOptions,
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_model_provider::history::{History, HistoryMessage, Role};
use flow_like_types::{async_trait, json::json};

#[derive(Default)]
pub struct GetSystemPromptNode {}

impl GetSystemPromptNode {
    pub fn new() -> Self {
        GetSystemPromptNode {}
    }
}

#[async_trait]
impl NodeLogic for GetSystemPromptNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "ai_generative_get_system_prompt",
            "Get System Prompt",
            "Gets the system prompt from a ChatHistory",
            "AI/Generative/History",
        );
        node.add_icon("/flow/icons/history.svg");

        node.add_input_pin("history", "History", "ChatHistory", VariableType::Struct)
            .set_schema::<History>()
            .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_output_pin(
            "system_prompt",
            "System Prompt",
            "System Prompt",
            VariableType::Struct,
        )
        .set_schema::<HistoryMessage>();

        node.add_output_pin(
            "success",
            "Found",
            "System Prompt Found",
            VariableType::Boolean,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let history: History = context.evaluate_pin("history").await?;
        let system_prompt = history.messages.iter().find_map(|message| {
            if message.role == Role::System {
                Some(message.clone())
            } else {
                None
            }
        });

        if let Some(system_prompt) = system_prompt {
            context.set_pin_value("success", json!(true)).await?;
            context
                .set_pin_value("system_prompt", json!(system_prompt))
                .await?;
            return Ok(());
        };

        context.set_pin_value("success", json!(false)).await?;
        Ok(())
    }
}
