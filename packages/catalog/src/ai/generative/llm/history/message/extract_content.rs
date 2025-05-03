use flow_like::{
    flow::{
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_model_provider::history::{Content, HistoryMessage, MessageContent};
use flow_like_types::{async_trait, json::json};

#[derive(Default)]
pub struct ExtractContentNode {}

impl ExtractContentNode {
    pub fn new() -> Self {
        ExtractContentNode {}
    }
}

#[async_trait]
impl NodeLogic for ExtractContentNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "ai_generative_message_extract_content",
            "Extract Content",
            "Get the String content from a Message",
            "AI/Generative/History/Message",
        );
        node.add_icon("/flow/icons/message.svg");

        node.add_input_pin("message", "Message", "Input Message", VariableType::Struct)
            .set_schema::<HistoryMessage>();

        node.add_output_pin("content", "Content", "Output Message", VariableType::String);

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let message: HistoryMessage = context.evaluate_pin("message").await?;
        let mut content: String = String::new();

        match message.content {
            MessageContent::String(text) => {
                content = text;
            }
            MessageContent::Contents(contents) => {
                for text in contents.iter() {
                    if let Content::Text { text, .. } = text {
                        content.push_str(&format!("{}\n", text));
                    }
                }
            }
        }

        context.set_pin_value("content", json!(content)).await?;

        Ok(())
    }
}
