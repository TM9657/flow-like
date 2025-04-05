use std::{collections::{HashMap, HashSet}, sync::{atomic::{AtomicUsize, Ordering}, Arc}, time::Duration};
use flow_like::{bit::{Bit, BitModelPreference, BitTypes}, flow::{board::Board, execution::{context::ExecutionContext, internal_node::InternalNode, log::{LogMessage, LogStat}, LogLevel}, node::{Node, NodeLogic}, pin::{PinOptions, PinType, ValueType}, variable::{Variable, VariableType}}, state::{FlowLikeState, ToastLevel}};
use flow_like_model_provider::{history::{History, HistoryMessage, Role}, llm::LLMCallback, response::{Response, ResponseMessage}, response_chunk::ResponseChunk, text_splitter::{ChunkConfig, MarkdownSplitter, TextSplitter}};
use flow_like_types::{anyhow, async_trait, json::{from_str, json, Deserialize, Serialize}, reqwest, sync::{DashMap, Mutex}, Bytes, Error, JsonSchema, Result, Value};
use nalgebra::DVector;
use regex::Regex;
use flow_like_storage::{object_store::PutPayload, Path};
use futures::StreamExt;
use crate::{ai::generative::embedding::{CachedEmbeddingModel, CachedEmbeddingModelObject}, storage::path::FlowPath, web::api::{HttpBody, HttpRequest, HttpResponse, Method}};

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

        node.add_output_pin(
            "failed",
            "Failed",
            "Failed to embed the query",
            VariableType::Execution,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) ->flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;
        context.activate_exec_pin("failed").await?;

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
        context.deactivate_exec_pin("failed").await?;

        Ok(())
    }
}
