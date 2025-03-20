pub mod local;
pub mod openai;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use text_splitter::{MarkdownSplitter, TextSplitter};

use crate::flow::execution::Cacheable;

#[async_trait]
pub trait EmbeddingModelLogic: Send + Sync + Cacheable + 'static {
    async fn get_splitter(
        &self,
        capacity: Option<usize>,
        overlap: Option<usize>,
    ) -> Result<(
        TextSplitter<tokenizers::Tokenizer>,
        MarkdownSplitter<tokenizers::Tokenizer>,
    )>;
    async fn text_embed_query(&self, texts: &Vec<String>) -> Result<Vec<Vec<f32>>>;
    async fn text_embed_document(&self, texts: &Vec<String>) -> Result<Vec<Vec<f32>>>;
    fn as_cacheable(&self) -> Arc<dyn Cacheable>;
}
