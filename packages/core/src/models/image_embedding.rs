pub mod local;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use text_splitter::{MarkdownSplitter, TextSplitter};

use crate::flow::execution::Cacheable;
#[async_trait]
pub trait ImageEmbeddingModelLogic: Send + Sync + Cacheable + 'static {
    async fn get_splitter(
        &self,
    ) -> anyhow::Result<(
        TextSplitter<tokenizers::Tokenizer>,
        MarkdownSplitter<tokenizers::Tokenizer>,
    )>;
    async fn text_embed_query(&self, texts: &Vec<String>) -> Result<Vec<Vec<f32>>>;
    async fn text_embed_document(&self, texts: &Vec<String>) -> Result<Vec<Vec<f32>>>;
    async fn image_embed(&self, image_paths: Vec<String>) -> Result<Vec<Vec<f32>>>;
    fn as_cacheable(&self) -> Arc<dyn Cacheable>;
}
