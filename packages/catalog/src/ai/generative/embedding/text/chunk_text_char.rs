use flow_like::{
    flow::{
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        pin::ValueType,
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_model_provider::text_splitter::{ChunkConfig, MarkdownSplitter, TextSplitter};
use flow_like_types::{async_trait, json::json};

#[crate::register_node]
#[derive(Default)]
pub struct ChunkTextChar {}

impl ChunkTextChar {
    pub fn new() -> Self {
        ChunkTextChar {}
    }
}

#[async_trait]
impl NodeLogic for ChunkTextChar {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "chunk_text_char",
            "Character Chunk Text",
            "For efficient embedding, chunk the text into smaller pieces",
            "AI/Preprocessing",
        );

        node.set_long_running(true);
        node.add_icon("/flow/icons/bot-invoke.svg");

        node.add_input_pin(
            "exec_in",
            "Input",
            "Initiate Execution",
            VariableType::Execution,
        );

        node.add_input_pin("text", "Text", "The string to embed", VariableType::String);

        node.add_input_pin(
            "capacity",
            "Capacity",
            "Chunk Capacity",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(512)));

        node.add_input_pin(
            "overlap",
            "Overlap",
            "Overlap between Chunks",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(20)));

        node.add_input_pin(
            "markdown",
            "Markdown",
            "Use Markdown Splitter?",
            VariableType::Boolean,
        )
        .set_default_value(Some(json!(true)));

        node.add_output_pin(
            "exec_out",
            "Output",
            "Done with the Execution",
            VariableType::Execution,
        );

        node.add_output_pin(
            "chunks",
            "Chunks",
            "The embedding vector",
            VariableType::String,
        )
        .set_value_type(ValueType::Array);

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;

        let text: String = context.evaluate_pin("text").await?;
        let capacity: i64 = context.evaluate_pin("capacity").await?;
        let overlap: i64 = context.evaluate_pin("overlap").await?;
        let markdown: bool = context.evaluate_pin("markdown").await?;

        let chunks = if markdown {
            let config = ChunkConfig::new(capacity as usize).with_overlap(overlap as usize)?;
            let splitter = TextSplitter::new(config);
            splitter
                .chunks(&text)
                .map(|c| c.to_string())
                .collect::<Vec<String>>()
        } else {
            let config = ChunkConfig::new(capacity as usize).with_overlap(overlap as usize)?;
            let splitter = MarkdownSplitter::new(config);
            splitter
                .chunks(&text)
                .map(|c| c.to_string())
                .collect::<Vec<String>>()
        };

        context.set_pin_value("chunks", json!(chunks)).await?;
        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }
}
