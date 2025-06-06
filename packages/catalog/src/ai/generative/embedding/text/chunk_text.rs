use crate::ai::generative::embedding::{CachedEmbeddingModel, CachedEmbeddingModelObject};
use flow_like::{
    flow::{
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        pin::{PinOptions, ValueType},
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_types::{anyhow, async_trait, bail, json::json};

#[derive(Default)]
pub struct ChunkText {}

impl ChunkText {
    pub fn new() -> Self {
        ChunkText {}
    }
}

#[async_trait]
impl NodeLogic for ChunkText {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "chunk_text",
            "Chunk Text",
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
            "model",
            "Model",
            "The embedding model",
            VariableType::Struct,
        )
        .set_schema::<CachedEmbeddingModel>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

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
        let model: CachedEmbeddingModel = context.evaluate_pin("model").await?;
        let capacity: i64 = context.evaluate_pin("capacity").await?;
        let overlap: i64 = context.evaluate_pin("overlap").await?;
        let markdown: bool = context.evaluate_pin("markdown").await?;

        let cached_model = context.get_cache(&model.cache_key).await;
        if cached_model.is_none() {
            bail!("Model not found in cache");
        }

        let cached_model = cached_model.unwrap();
        let embedding_model = cached_model
            .as_any()
            .downcast_ref::<CachedEmbeddingModelObject>()
            .ok_or(anyhow!("Failed to Downcast Model"))?;

        let (text_splitter, md_splitter) =
            if let Some(text_model) = embedding_model.text_model.clone() {
                text_model
                    .get_splitter(Some(capacity as usize), Some(overlap as usize))
                    .await?
            } else if let Some(image_model) = embedding_model.image_model.clone() {
                image_model
                    .get_splitter(Some(capacity as usize), Some(overlap as usize))
                    .await?
            } else {
                return Err(anyhow!("No model found"));
            };

        let chunks = if markdown {
            md_splitter.chunks(&text)
        } else {
            text_splitter.chunks(&text)
        }?;

        context.set_pin_value("chunks", json!(chunks)).await?;
        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }
}
