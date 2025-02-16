pub mod local;
use anyhow::Result;
use async_trait::async_trait;
use text_splitter::{MarkdownSplitter, TextSplitter};
#[async_trait]
pub trait ImageEmbeddingModelLogic: Send + Sync {
    async fn get_splitter(
        &self,
    ) -> anyhow::Result<(
        TextSplitter<tokenizers::Tokenizer>,
        MarkdownSplitter<tokenizers::Tokenizer>,
    )>;
    async fn text_embed_query(&self, texts: &Vec<String>) -> Result<Vec<Vec<f32>>>;
    async fn text_embed_document(&self, texts: &Vec<String>) -> Result<Vec<Vec<f32>>>;
    async fn image_embed(&self, image_paths: Vec<String>) -> Result<Vec<Vec<f32>>>;
}
