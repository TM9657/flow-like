use flow_like_types::Cacheable;
use flow_like_types::Result;
use flow_like_types::async_trait;
use std::sync::Arc;

use crate::embedding::GeneralTextSplitter;

#[async_trait]
pub trait ImageEmbeddingModelLogic: Send + Sync + Cacheable + 'static {
    async fn get_splitter(
        &self,
        capacity: Option<usize>,
        overlap: Option<usize>,
    ) -> flow_like_types::Result<(GeneralTextSplitter, GeneralTextSplitter)>;
    async fn text_embed_query(&self, texts: &Vec<String>) -> Result<Vec<Vec<f32>>>;
    async fn text_embed_document(&self, texts: &Vec<String>) -> Result<Vec<Vec<f32>>>;
    async fn image_embed(&self, image_paths: Vec<String>) -> Result<Vec<Vec<f32>>>;
    fn as_cacheable(&self) -> Arc<dyn Cacheable>;
}
