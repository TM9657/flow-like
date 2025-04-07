use flow_like_types::Cacheable;
use flow_like_types::Result;
use flow_like_types::async_trait;
use std::sync::Arc;
use text_splitter::Characters;
use text_splitter::{MarkdownSplitter, TextSplitter};
use tiktoken_rs::CoreBPE;

pub mod openai;

#[derive(Clone)]
pub enum GeneralTextSplitter {
    MarkdownCharacter(Arc<MarkdownSplitter<Characters>>),
    TextCharacters(Arc<TextSplitter<Characters>>),
    MarkdownTokenizer(Arc<MarkdownSplitter<tokenizers::Tokenizer>>),
    TextTokenizer(Arc<TextSplitter<tokenizers::Tokenizer>>),
    MarkdownTiktoken(Arc<MarkdownSplitter<Arc<CoreBPE>>>),
    TextTiktoken(Arc<TextSplitter<Arc<CoreBPE>>>),
}

impl GeneralTextSplitter {
    pub fn chunks(&self, text: &str) -> Result<Vec<String>> {
        Ok(match self {
            GeneralTextSplitter::MarkdownCharacter(splitter) => {
                splitter.chunks(text).map(|f| f.to_string()).collect()
            }
            GeneralTextSplitter::TextCharacters(splitter) => {
                splitter.chunks(text).map(|f| f.to_string()).collect()
            }
            GeneralTextSplitter::MarkdownTokenizer(splitter) => {
                splitter.chunks(text).map(|f| f.to_string()).collect()
            }
            GeneralTextSplitter::TextTokenizer(splitter) => {
                splitter.chunks(text).map(|f| f.to_string()).collect()
            }
            GeneralTextSplitter::MarkdownTiktoken(splitter) => {
                splitter.chunks(text).map(|f| f.to_string()).collect()
            }
            GeneralTextSplitter::TextTiktoken(splitter) => {
                splitter.chunks(text).map(|f| f.to_string()).collect()
            }
        })
    }
}

#[async_trait]
pub trait EmbeddingModelLogic: Send + Sync + Cacheable + 'static {
    async fn get_splitter(
        &self,
        capacity: Option<usize>,
        overlap: Option<usize>,
    ) -> Result<(GeneralTextSplitter, GeneralTextSplitter)>;
    async fn text_embed_query(&self, texts: &Vec<String>) -> Result<Vec<Vec<f32>>>;
    async fn text_embed_document(&self, texts: &Vec<String>) -> Result<Vec<Vec<f32>>>;
    fn as_cacheable(&self) -> Arc<dyn Cacheable>;
}
